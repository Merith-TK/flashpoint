// boot-rom — Flashpoint OS kernel
//
// Hardware-agnostic. Zero hardware code here. All I/O goes through the
// Platform trait pointer published by flash-rom at PLATFORM_PTR_ADDR
// before jumping to this entry point.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

extern crate alloc;

pub mod kernel;
pub mod shell;
pub mod runtime;

use flashpoint_common::{Event, PLATFORM_PTR_ADDR};

// ─── Entry point ─────────────────────────────────────────────────────────────

/// Called by Stage 1 after it validates and loads this binary.
/// Reads the Platform vtable pointer from the agreed handoff address,
/// then passes it to the OS init chain.
#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn entry() -> ! {
    // SAFETY: Stage 1 wrote a valid fat-pointer to PLATFORM_PTR_ADDR
    // before jumping here. Both crates agree on the address via flashpoint-common.
    let platform = unsafe {
        // The pointer written is a *const dyn Platform fat-pointer (2 words).
        // We read it back as a trait object reference.
        // Concrete type lives in flash-rom; we only have the vtable here.
        &*(*(PLATFORM_PTR_ADDR as *const *const ()))
            as *const dyn PlatformShim
    };

    // TODO (step 0.5): call platform.display_clear() and render boot screen
    // For now just loop — proves the entry point links correctly.
    loop {
        // poll_event → react → sleep
        let _ = platform;
        unsafe { core::hint::spin_loop() }
    }
}

// Shim trait: boot-rom does not depend on flash-rom crate directly.
// It receives the Platform vtable through the fat-pointer handoff.
// A proper trait alias will replace this in step 0.5 once the Platform
// trait is moved to flashpoint-common (or linked via an ABI shim).
trait PlatformShim {}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
