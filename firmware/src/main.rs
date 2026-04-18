// firmware — device firmware burned once to internal flash.
//
// Contains: Stage 1 chainloader, all hardware drivers, Platform trait impl.
// Loads the kernel (flashpoint.rom) from SD card or internal flash,
// then jumps to it after publishing the Platform vtable pointer.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

extern crate alloc;

pub mod capabilities;
pub mod hal;
mod stage1;

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
