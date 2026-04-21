pub const MAGIC: [u8; 4] = *b"FLPT";
pub const HEADER_END_MAGIC: [u8; 4] = *b"FLPE";
pub const HEADER_V1_SIZE: usize = 64;

pub const OFF_MAGIC: usize = 0x00;
pub const OFF_PLATFORM: usize = 0x04;
pub const OFF_ROM_VERSION: usize = 0x05;
pub const OFF_BUILT_AGAINST: usize = 0x08;
pub const OFF_FLAGS: usize = 0x0C;
pub const OFF_REQUIRED_FEATURES: usize = 0x0E;
pub const OFF_PAYLOAD_LEN: usize = 0x16;
pub const OFF_CRC32: usize = 0x1A;
pub const OFF_PAYLOAD_TYPE: usize = 0x1E;
pub const OFF_ROM_ID: usize = 0x1F;
pub const OFF_COMPAT_PLATFORMS: usize = 0x37;
pub const OFF_HEADER_SIZE: usize = 0x3A;
pub const OFF_HEADER_END: usize = 0x3C;

pub const ROM_ID_LEN: usize = 24;

pub const PLATFORM_ESP32: u8 = 0x01;
pub const PLATFORM_ESP32S3: u8 = 0x02;
pub const PLATFORM_RP2040: u8 = 0x03;
pub const PLATFORM_ANY: u8 = 0xFF;

pub const FEAT_WIFI: u64 = 1 << 0;
pub const FEAT_BLE: u64 = 1 << 1;
pub const FEAT_USB_OTG: u64 = 1 << 2;
pub const FEAT_DISP_TFT: u64 = 1 << 8;
pub const FEAT_DISP_EINK: u64 = 1 << 9;
pub const FEAT_INPUT_TOUCH: u64 = 1 << 16;
pub const FEAT_INPUT_BUTTONS: u64 = 1 << 17;
pub const FEAT_PSRAM: u64 = 1 << 24;
pub const FEAT_BATTERY: u64 = 1 << 25;

pub const PLATFORM_PTR_ADDR: usize = 0x3FFB_0000;
