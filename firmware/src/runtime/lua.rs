/// Lua runtime stub.
///
/// Full Lua execution is not yet implemented.  For now, firmware logs a
/// message and drops back to recovery so the device remains usable.
use common::Platform;

pub fn boot(platform: &dyn Platform) -> ! {
    log::warn!("[lua] Lua runtime not yet implemented — entering recovery");
    common::recovery_main(platform)
}
