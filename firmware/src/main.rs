// firmware — device firmware burned once to internal flash.
//
// Contains: Stage 1 chainloader, all hardware drivers, Platform trait impl.
// Loads the kernel (flashpoint.rom) from SD card or internal flash,
// then jumps to it after publishing the Platform vtable pointer.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

extern crate alloc;

pub mod hal;
mod stage1;

// CYD (ESP32-2432S028R) device capabilities:
//   ILI9341 TFT display + XPT2046 resistive touch
//   No PSRAM, no WiFi in base firmware (WiFi extension is Phase 4)
pub const DEVICE_FEATURES: u64 =
    common::FEAT_DISPLAY_TFT |
    common::FEAT_INPUT_TOUCH;

#[cfg(not(test))]
#[no_mangle]
extern "C" fn app_main() {
    stage1::stage1_main()
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
