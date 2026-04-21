use common::Event;
use esp_idf_svc::hal::gpio::{Input, Output, PinDriver};

pub(crate) type OutPin = PinDriver<'static, Output>;
pub(crate) type InPin = PinDriver<'static, Input>;

// ─── Touch calibration data ──────────────────────────────────────────────────

/// Calibration data for the XPT2046 touch controller.
///
/// Stored in NVS namespace `"fp-hal"`, key `"touch-cal"`, 8 bytes:
/// `[x_min_lo, x_min_hi, x_max_lo, x_max_hi, y_min_lo, y_min_hi, y_max_lo, y_max_hi]`.
///
/// Defaults to the full 12-bit ADC range; proportional zone math with these defaults
/// reproduces the prior hardcoded thresholds (≈1000 / 1200 / 2800 / 3100).
#[derive(Clone, Copy)]
pub(crate) struct TouchCal {
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
    pub(crate) fn from_bytes(b: &[u8]) -> Option<Self> {
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

pub(crate) struct TouchBitbang {
    pub(crate) clk: OutPin,
    pub(crate) mosi: OutPin,
    pub(crate) miso: InPin,
    pub(crate) cs: OutPin,
    pub(crate) irq: InPin,
    pub(crate) last_event: Option<Event>,
    pub(crate) debounce_ms: u32,
    pub(crate) cal: TouchCal,
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
    pub(crate) fn sample(&mut self) -> Option<(u16, u16)> {
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
    pub(crate) fn map_to_event(raw_x: u16, raw_y: u16, cal: &TouchCal) -> Option<Event> {
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
