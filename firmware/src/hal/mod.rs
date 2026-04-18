#[cfg(feature = "board-cyd")]
pub mod esp32_cyd;
#[cfg(feature = "board-qemu")]
pub mod emulator;

pub use common::{FrameBuffer, Platform, PlatformError};

#[cfg(feature = "board-cyd")]
pub use esp32_cyd::CydPlatform as ActivePlatform;
#[cfg(feature = "board-qemu")]
pub use emulator::EmulatorPlatform as ActivePlatform;
