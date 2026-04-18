// Platform trait lives in flashpoint-common so both flash-rom and emulator
// can implement it without a circular dependency.
pub use flashpoint_common::{FrameBuffer, Platform, PlatformError};
