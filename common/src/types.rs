use crate::constants::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PayloadType {
    Native = 0x00,
    Wasm32 = 0x01,
    Luac54 = 0x02,
}

impl PayloadType {
    pub fn from_u8(b: u8) -> Option<Self> {
        match b {
            0x00 => Some(Self::Native),
            0x01 => Some(Self::Wasm32),
            0x02 => Some(Self::Luac54),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Wasm32 => "wasm32",
            Self::Luac54 => "luac54",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipId {
    Esp32,
    Esp32S3,
    Rp2040,
}

impl ChipId {
    pub fn platform_byte(self) -> u8 {
        match self {
            ChipId::Esp32 => PLATFORM_ESP32,
            ChipId::Esp32S3 => PLATFORM_ESP32S3,
            ChipId::Rp2040 => PLATFORM_RP2040,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    BtnUp,
    BtnDown,
    BtnLeft,
    BtnRight,
    BtnSelect,
    BtnBack,
    BatteryLow,
    HibernateWarning,
}

pub struct FrameBuffer<'a> {
    pub y: u16,
    pub data: &'a [u8],
}
