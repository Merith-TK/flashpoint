// flash-rom — device firmware burned once to internal flash.
//
// Contains: Stage 1 chainload logic, all hardware drivers, Platform trait impl.
// The boot-rom binary it loads is hardware-agnostic and calls back through
// the Platform trait pointer published at PLATFORM_PTR_ADDR.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

extern crate alloc;

pub mod capabilities;
pub mod hal;

// TODO (step 0.5): wire up stage1 entry once esp-idf-sys is linked
// For now this compiles as a library for type-checking purposes.

#[cfg(not(test))]
#[no_mangle]
extern "C" fn app_main() {
    // Stage 1 entry — implemented in stage1/src/main.rs.
    // flash-rom/src/main.rs is the top-level crate that links everything.
    // step 0.5 will wire the HAL init and stage1_main() call here.
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
