// hal-cyd — CYD (ESP32-2432S028R) Platform implementation
//
// Verified pin assignments from witnessmenow/ESP32-Cheap-Yellow-Display PINS.md:
//
//   LCD (HSPI / SPI2):
//     TFT_MOSI = IO13    TFT_MISO = IO12    TFT_SCK = IO14
//     TFT_CS   = IO15    TFT_DC   = IO2     TFT_BL  = IO21
//
//   Touch (XPT2046 — bit-bang SPI, separate from LCD):
//     XPT2046_CLK  = IO25    XPT2046_MOSI = IO32    XPT2046_MISO = IO39
//     XPT2046_CS   = IO33    XPT2046_IRQ  = IO36
//
//   SD card (VSPI / SPI3):
//     SD_MOSI = IO23    SD_MISO = IO19    SD_SCK = IO18    SD_SS = IO5
//
//   RGB LED (active LOW — HIGH = off, LOW = on):
//     R = IO4    G = IO16    B = IO17
//
// Boot button (used as recovery trigger):
//   BOOT = IO0 (active LOW, internal pull-up)

use common::{
    ChipId, Event, FrameBuffer, Platform, PlatformError, FEAT_BLE, FEAT_DISP_TFT, FEAT_INPUT_TOUCH,
    FEAT_WIFI, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING,
};

use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::{AnyOutputPin, Input, Output, PinDriver, Pull},
    spi::{SpiConfig, SpiDeviceDriver, SpiDriver, SpiDriverConfig, SPI2, SPI3},
    units::FromValueType,
};
use esp_idf_svc::sys as idf;

use display_interface_spi::SPIInterface;
use embedded_graphics_core::{
    draw_target::DrawTarget,
    pixelcolor::{raw::RawU16, Rgb565},
};
use embedded_sdmmc::{BlockCount, BlockDevice, BlockIdx, SdCard};
use mipidsi::{
    models::ILI9341Rgb565,
    options::{ColorOrder, Orientation},
    Builder,
};

use std::ffi::CString;
use std::sync::Mutex;
use std::vec::Vec;

// ─── Type aliases ─────────────────────────────────────────────────────────────

type LcdSpiDriver = SpiDriver<'static>;
type LcdSpiDevice = SpiDeviceDriver<'static, LcdSpiDriver>;
type LcdDcPin = PinDriver<'static, Output>;
type LcdInterface = SPIInterface<LcdSpiDevice, LcdDcPin>;
type LcdDisplay = mipidsi::Display<LcdInterface, ILI9341Rgb565, mipidsi::NoResetPin>;

type SdSpiDriver = SpiDriver<'static>;
type SdSpiDevice = SpiDeviceDriver<'static, SdSpiDriver>;
type SdCsPin = PinDriver<'static, Output>;
type SdCardDev = SdCard<SdSpiDevice, SdCsPin, FreeRtos>;

type OutPin = PinDriver<'static, Output>;
type InPin = PinDriver<'static, Input>;

// ─── Touch calibration data ──────────────────────────────────────────────────

/// Calibration data for the XPT2046 touch controller.
///
/// Stored in NVS namespace `"fp-hal"`, key `"touch-cal"`, 8 bytes:
/// `[x_min_lo, x_min_hi, x_max_lo, x_max_hi, y_min_lo, y_min_hi, y_max_lo, y_max_hi]`.
///
/// Defaults to the full 12-bit ADC range; proportional zone math with these defaults
/// reproduces the prior hardcoded thresholds (≈1000 / 1200 / 2800 / 3100).
#[derive(Clone, Copy)]
struct TouchCal {
    x_min: u16,
    x_max: u16,
    y_min: u16,
    y_max: u16,
}

impl Default for TouchCal {
    fn default() -> Self {
        TouchCal {
            x_min: 0,
            x_max: 4095,
            y_min: 0,
            y_max: 4095,
        }
    }
}

impl TouchCal {
    fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() < 8 {
            return None;
        }
        Some(TouchCal {
            x_min: u16::from_le_bytes([b[0], b[1]]),
            x_max: u16::from_le_bytes([b[2], b[3]]),
            y_min: u16::from_le_bytes([b[4], b[5]]),
            y_max: u16::from_le_bytes([b[6], b[7]]),
        })
    }
}

// ─── Internal touch state ─────────────────────────────────────────────────────

struct TouchBitbang {
    clk: OutPin,
    mosi: OutPin,
    miso: InPin,
    cs: OutPin,
    irq: InPin,
    last_event: Option<Event>,
    debounce_ms: u32,
    cal: TouchCal,
}

impl TouchBitbang {
    /// Read a 12-bit value from XPT2046 for the given channel command byte.
    /// Bit-bangs a full SPI transaction (CS low → 8-bit cmd → 16-bit response → CS high).
    fn read_channel(&mut self, cmd: u8) -> u16 {
        self.cs.set_low().ok();

        // Send 8-bit command
        for bit in (0..8).rev() {
            self.mosi.set_level(((cmd >> bit) & 1 != 0).into()).ok();
            self.clk.set_high().ok();
            self.clk.set_low().ok();
        }

        // Read 16-bit response (only top 12 bits valid; result = raw >> 3)
        let mut raw: u16 = 0;
        for _ in 0..16 {
            self.clk.set_high().ok();
            raw = (raw << 1) | (self.miso.is_high() as u16);
            self.clk.set_low().ok();
        }

        self.cs.set_high().ok();
        (raw >> 3) & 0x0FFF
    }

    /// Sample touch. Returns Some((raw_x, raw_y)) if screen is touched, None otherwise.
    fn sample(&mut self) -> Option<(u16, u16)> {
        if self.irq.is_high() {
            return None; // IRQ active-low; high = no touch
        }
        let x = self.read_channel(0xD0); // X channel
        let y = self.read_channel(0x90); // Y channel
        Some((x, y))
    }

    /// Map raw XPT2046 values to an Event.
    ///
    /// Zone boundaries are proportional to the calibrated range:
    /// each axis is divided into three bands — the outer quarters trigger the
    /// directional events (Up/Down/Left/Right) and the centre half triggers
    /// BtnSelect.  With default cal (0–4095) the boundaries are ≈1023/3071,
    /// matching the prior hardcoded thresholds.
    fn map_to_event(raw_x: u16, raw_y: u16, cal: &TouchCal) -> Option<Event> {
        let xr = (cal.x_max as u32).saturating_sub(cal.x_min as u32);
        let yr = (cal.y_max as u32).saturating_sub(cal.y_min as u32);
        let x_lo = cal.x_min.saturating_add((xr / 4) as u16);
        let x_hi = cal.x_max.saturating_sub((xr / 4) as u16);
        let y_lo = cal.y_min.saturating_add((yr / 4) as u16);
        let y_hi = cal.y_max.saturating_sub((yr / 4) as u16);

        if raw_y < y_lo {
            Some(Event::BtnUp)
        } else if raw_y > y_hi {
            Some(Event::BtnDown)
        } else if raw_x < x_lo {
            Some(Event::BtnLeft)
        } else if raw_x > x_hi {
            Some(Event::BtnRight)
        } else {
            Some(Event::BtnSelect)
        }
    }
}

// ─── NVS helpers (raw esp-idf-sys C bindings) ────────────────────────────────

fn nvs_get(ns: &str, key: &str) -> Result<Vec<u8>, PlatformError> {
    unsafe {
        let ns_c = CString::new(ns).map_err(|_| PlatformError::NvsError)?;
        let key_c = CString::new(key).map_err(|_| PlatformError::NvsError)?;

        let mut handle: idf::nvs_handle_t = 0;
        idf::nvs_open(
            ns_c.as_ptr(),
            idf::nvs_open_mode_t_NVS_READONLY,
            &mut handle,
        );

        let mut size: usize = 0;
        let rc = idf::nvs_get_blob(handle, key_c.as_ptr(), core::ptr::null_mut(), &mut size);
        if rc != idf::ESP_OK {
            idf::nvs_close(handle);
            return Err(PlatformError::NvsError);
        }

        let mut buf = vec![0u8; size];
        let rc = idf::nvs_get_blob(
            handle,
            key_c.as_ptr(),
            buf.as_mut_ptr() as *mut _,
            &mut size,
        );
        idf::nvs_close(handle);

        if rc == idf::ESP_OK {
            Ok(buf)
        } else {
            Err(PlatformError::NvsError)
        }
    }
}

fn nvs_set(ns: &str, key: &str, val: &[u8]) -> Result<(), PlatformError> {
    unsafe {
        let ns_c = CString::new(ns).map_err(|_| PlatformError::NvsError)?;
        let key_c = CString::new(key).map_err(|_| PlatformError::NvsError)?;

        let mut handle: idf::nvs_handle_t = 0;
        let rc = idf::nvs_open(
            ns_c.as_ptr(),
            idf::nvs_open_mode_t_NVS_READWRITE,
            &mut handle,
        );
        if rc != idf::ESP_OK {
            return Err(PlatformError::NvsError);
        }

        let rc = idf::nvs_set_blob(handle, key_c.as_ptr(), val.as_ptr() as *const _, val.len());
        if rc == idf::ESP_OK {
            idf::nvs_commit(handle);
        }
        idf::nvs_close(handle);

        if rc == idf::ESP_OK {
            Ok(())
        } else {
            Err(PlatformError::NvsError)
        }
    }
}

fn nvs_erase(ns: &str, key: &str) -> Result<(), PlatformError> {
    unsafe {
        let ns_c = CString::new(ns).map_err(|_| PlatformError::NvsError)?;
        let key_c = CString::new(key).map_err(|_| PlatformError::NvsError)?;

        let mut handle: idf::nvs_handle_t = 0;
        let rc = idf::nvs_open(
            ns_c.as_ptr(),
            idf::nvs_open_mode_t_NVS_READWRITE,
            &mut handle,
        );
        if rc != idf::ESP_OK {
            return Err(PlatformError::NvsError);
        }

        let rc = idf::nvs_erase_key(handle, key_c.as_ptr());
        if rc == idf::ESP_OK {
            idf::nvs_commit(handle);
        }
        idf::nvs_close(handle);

        if rc == idf::ESP_OK {
            Ok(())
        } else {
            Err(PlatformError::NvsError)
        }
    }
}

// ─── CydPlatform ─────────────────────────────────────────────────────────────

pub struct CydPlatform {
    display: Mutex<LcdDisplay>,
    #[allow(dead_code)] // must be held to keep backlight pin driven high
    backlight: Mutex<OutPin>,
    touch: Mutex<TouchBitbang>,
    sd_card: SdCardDev, // SdCard uses RefCell internally; safe via &self
    led_r: Mutex<OutPin>,
    led_g: Mutex<OutPin>,
    led_b: Mutex<OutPin>,
}

// SAFETY: CydPlatform is used from a single FreeRTOS task in the boot-rom.
// The Mutex guards provide safe access to drivers that require &mut self.
unsafe impl Send for CydPlatform {}
unsafe impl Sync for CydPlatform {}

impl CydPlatform {
    /// Initialise all CYD peripherals and return a ready-to-use platform.
    ///
    /// Takes `Peripherals` by value (moved in), so this can only be called once.
    pub fn new(peripherals: esp_idf_svc::hal::peripherals::Peripherals) -> Self {
        let pins = peripherals.pins;

        // ── LCD (HSPI / SPI2) ─────────────────────────────────────────────────
        let lcd_driver = SpiDriver::new::<SPI2>(
            peripherals.spi2,
            pins.gpio14,       // CLK
            pins.gpio13,       // MOSI
            Some(pins.gpio12), // MISO
            &SpiDriverConfig::new(),
        )
        .expect("LCD SPI driver init failed");

        // SAFETY: hardware peripheral lives for the program's lifetime;
        // extending the phantom lifetime to 'static is correct here.
        let lcd_driver: LcdSpiDriver = unsafe { core::mem::transmute(lcd_driver) };

        let lcd_device = SpiDeviceDriver::new(
            lcd_driver,
            Some(pins.gpio15.degrade_output()), // CS
            &SpiConfig::new().baudrate(40u32.MHz().into()),
        )
        .expect("LCD SPI device init failed");
        let lcd_device: LcdSpiDevice = unsafe { core::mem::transmute(lcd_device) };

        let lcd_dc: LcdDcPin = unsafe {
            core::mem::transmute(
                PinDriver::output(pins.gpio2.degrade_output()).expect("LCD DC pin init failed"),
            )
        };

        let mut backlight: OutPin = unsafe {
            core::mem::transmute(
                PinDriver::output(pins.gpio21.degrade_output()).expect("LCD BL pin init failed"),
            )
        };

        let di = SPIInterface::new(lcd_device, lcd_dc);
        let mut delay = FreeRtos;
        let display = Builder::new(ILI9341Rgb565, di)
            .display_size(240, 320)
            .color_order(ColorOrder::Bgr)
            .orientation(Orientation::default())
            .init(&mut delay)
            .expect("ILI9341 init failed");

        // Enable backlight (active high)
        backlight.set_high().ok();

        // ── Touch (XPT2046 — bit-bang SPI) ────────────────────────────────────
        let touch_clk: OutPin = unsafe {
            core::mem::transmute(
                PinDriver::output(pins.gpio25.degrade_output()).expect("touch CLK init failed"),
            )
        };
        let touch_mosi: OutPin = unsafe {
            core::mem::transmute(
                PinDriver::output(pins.gpio32.degrade_output()).expect("touch MOSI init failed"),
            )
        };
        let touch_miso: InPin = unsafe {
            core::mem::transmute(
                PinDriver::input(pins.gpio39.degrade_input(), Pull::Floating)
                    .expect("touch MISO init failed"),
            )
        };
        let touch_cs: OutPin = unsafe {
            core::mem::transmute(
                PinDriver::output(pins.gpio33.degrade_output()).expect("touch CS init failed"),
            )
        };
        let touch_irq: InPin = unsafe {
            core::mem::transmute(
                PinDriver::input(pins.gpio36.degrade_input(), Pull::Up)
                    .expect("touch IRQ init failed"),
            )
        };

        // Load touch calibration from NVS (written by recovery TOUCH CALIBRATION).
        // Falls back to full-range defaults if no calibration has been saved yet.
        let touch_cal = match nvs_get("fp-hal", "touch-cal") {
            Ok(bytes) => TouchCal::from_bytes(&bytes).unwrap_or_default(),
            Err(_) => {
                log::info!("[hal-cyd] no touch calibration in NVS, using defaults");
                TouchCal::default()
            }
        };

        let touch = TouchBitbang {
            clk: touch_clk,
            mosi: touch_mosi,
            miso: touch_miso,
            cs: touch_cs,
            irq: touch_irq,
            last_event: None,
            debounce_ms: 0,
            cal: touch_cal,
        };

        // ── SD card (VSPI / SPI3) ──────────────────────────────────────────────
        let sd_driver = SpiDriver::new::<SPI3>(
            peripherals.spi3,
            pins.gpio18,       // CLK
            pins.gpio23,       // MOSI
            Some(pins.gpio19), // MISO
            &SpiDriverConfig::new(),
        )
        .expect("SD SPI driver init failed");
        let sd_driver: SdSpiDriver = unsafe { core::mem::transmute(sd_driver) };

        let sd_device = SpiDeviceDriver::new(
            sd_driver,
            None::<AnyOutputPin>, // CS managed by SdCard separately
            &SpiConfig::new().baudrate(400u32.kHz().into()), // start slow for card init
        )
        .expect("SD SPI device init failed");
        let sd_device: SdSpiDevice = unsafe { core::mem::transmute(sd_device) };

        let sd_cs: SdCsPin = unsafe {
            core::mem::transmute(
                PinDriver::output(pins.gpio5.degrade_output()).expect("SD CS pin init failed"),
            )
        };

        let sd_card = SdCard::new(sd_device, sd_cs, FreeRtos);

        // ── RGB LED (active LOW) ───────────────────────────────────────────────
        let mut led_r: OutPin = unsafe {
            core::mem::transmute(
                PinDriver::output(pins.gpio4.degrade_output()).expect("LED R init failed"),
            )
        };
        let mut led_g: OutPin = unsafe {
            core::mem::transmute(
                PinDriver::output(pins.gpio16.degrade_output()).expect("LED G init failed"),
            )
        };
        let mut led_b: OutPin = unsafe {
            core::mem::transmute(
                PinDriver::output(pins.gpio17.degrade_output()).expect("LED B init failed"),
            )
        };

        // LEDs off by default (active low → HIGH = off)
        led_r.set_high().ok();
        led_g.set_high().ok();
        led_b.set_high().ok();

        CydPlatform {
            display: Mutex::new(display),
            backlight: Mutex::new(backlight),
            touch: Mutex::new(touch),
            sd_card,
            led_r: Mutex::new(led_r),
            led_g: Mutex::new(led_g),
            led_b: Mutex::new(led_b),
        }
    }
}

// ─── Platform trait implementation ───────────────────────────────────────────

impl Platform for CydPlatform {
    // ── Display ───────────────────────────────────────────────────────────────

    fn display_flush(&self, buf: &FrameBuffer) -> Result<(), PlatformError> {
        let mut display = self.display.lock().unwrap();
        let pixels = buf
            .data
            .chunks_exact(2)
            .map(|c| Rgb565::from(RawU16::new(u16::from_le_bytes([c[0], c[1]]))));
        display
            .set_pixels(0, buf.y, self.display_width() - 1, buf.y, pixels)
            .map_err(|_| PlatformError::DisplayError)
    }

    fn display_clear(&self) -> Result<(), PlatformError> {
        let mut display = self.display.lock().unwrap();
        display
            .clear(Rgb565::from(RawU16::new(0x0000)))
            .map_err(|_| PlatformError::DisplayError)
    }

    fn display_width(&self) -> u16 {
        240
    }
    fn display_height(&self) -> u16 {
        320
    }

    // ── Input (XPT2046 via bit-bang SPI) ─────────────────────────────────────

    fn poll_event(&self) -> Option<Event> {
        let mut touch = self.touch.lock().unwrap();
        match touch.sample() {
            None => {
                touch.last_event = None;
                touch.debounce_ms = 0;
                None
            }
            Some((x, y)) => {
                let event = TouchBitbang::map_to_event(x, y, &touch.cal);
                if event == touch.last_event {
                    // Still same zone — only emit once per touch (debounce)
                    None
                } else {
                    touch.last_event = event;
                    touch.debounce_ms = 50;
                    event
                }
            }
        }
    }

    fn poll_touch_xy(&self) -> Option<(u16, u16)> {
        self.touch.lock().unwrap().sample()
    }

    // ── RGB LED (active LOW) ──────────────────────────────────────────────────

    fn led_rgb(&self, r: u8, g: u8, b: u8) -> Result<(), PlatformError> {
        // Active low: value > 0 → pull pin LOW (on), 0 → HIGH (off)
        self.led_r
            .lock()
            .unwrap()
            .set_level((r == 0).into())
            .map_err(|_| PlatformError::NotSupported)?;
        self.led_g
            .lock()
            .unwrap()
            .set_level((g == 0).into())
            .map_err(|_| PlatformError::NotSupported)?;
        self.led_b
            .lock()
            .unwrap()
            .set_level((b == 0).into())
            .map_err(|_| PlatformError::NotSupported)?;
        Ok(())
    }

    // ── Storage (SD card) ─────────────────────────────────────────────────────

    fn sd_read_sectors(&self, start: u32, buf: &mut [u8]) -> Result<(), PlatformError> {
        let num = buf.len() / 512;
        let mut blocks = vec![embedded_sdmmc::Block::new(); num];
        self.sd_card
            .read(&mut blocks, BlockIdx(start), "read")
            .map_err(|_| PlatformError::SdReadError)?;
        for (i, blk) in blocks.iter().enumerate() {
            buf[i * 512..(i + 1) * 512].copy_from_slice(&blk.contents);
        }
        Ok(())
    }

    fn sd_write_sectors(&self, start: u32, buf: &[u8]) -> Result<(), PlatformError> {
        let num = buf.len() / 512;
        let mut blocks = vec![embedded_sdmmc::Block::new(); num];
        for (i, blk) in blocks.iter_mut().enumerate() {
            blk.contents.copy_from_slice(&buf[i * 512..(i + 1) * 512]);
        }
        self.sd_card
            .write(&blocks, BlockIdx(start))
            .map_err(|_| PlatformError::SdWriteError)
    }

    fn sd_sector_count(&self) -> u32 {
        self.sd_card
            .num_blocks()
            .map(|BlockCount(n)| n)
            .unwrap_or(0)
    }

    // ── NVS ───────────────────────────────────────────────────────────────────

    fn nvs_read(&self, ns: &str, key: &str) -> Result<Vec<u8>, PlatformError> {
        nvs_get(ns, key)
    }
    fn nvs_write(&self, ns: &str, key: &str, val: &[u8]) -> Result<(), PlatformError> {
        nvs_set(ns, key, val)
    }
    fn nvs_delete(&self, ns: &str, key: &str) -> Result<(), PlatformError> {
        nvs_erase(ns, key)
    }

    // ── System ────────────────────────────────────────────────────────────────

    fn battery_percent(&self) -> u8 {
        100
    } // CYD has no battery

    fn chip_id(&self) -> ChipId {
        ChipId::Esp32
    }

    fn reboot(&self) -> ! {
        unsafe { idf::esp_restart() }
    }

    fn sleep_ms(&self, ms: u32) {
        FreeRtos::delay_ms(ms);
    }

    fn flashpoint_version(&self) -> (u32, u32) {
        (FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING)
    }

    fn wasm_arena_limit(&self) -> usize {
        256 * 1024
    }
    fn lua_heap_limit(&self) -> usize {
        64 * 1024
    }

    // ── Capabilities ──────────────────────────────────────────────────────────

    fn features(&self) -> u64 {
        FEAT_DISP_TFT | FEAT_INPUT_TOUCH | FEAT_WIFI | FEAT_BLE
    }
}
