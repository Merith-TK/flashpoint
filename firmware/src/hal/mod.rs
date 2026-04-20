#[cfg(feature = "board-cyd")]
pub use hal_cyd::CydPlatform as ActivePlatform;
#[cfg(feature = "board-qemu")]
pub use hal_qemu::EmulatorPlatform as ActivePlatform;

pub use common::{FrameBuffer, Platform, PlatformError};
