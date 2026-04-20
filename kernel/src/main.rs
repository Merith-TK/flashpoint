// kernel — Flashpoint OS kernel
//
// Hardware-agnostic. Zero hardware code here. All I/O goes through the
// Platform trait pointer published by firmware at PLATFORM_PTR_ADDR
// before jumping to this entry point.
//
// Future sub-modules (added when implemented):
//   mod kernel;   — Phase 1: card-ram paging, FatFS, NVS, event loop
//   mod shell;    — Phase 3: status bar, battery bar, app grid, dropdown
//   mod runtime;  — Phase 2: wasm3 integration, Lua 5.4 VM, host API

use common::{Platform, PLATFORM_PTR_ADDR};

// ─── Entry point (jumped to by Stage 1 on real hardware) ─────────────────────

#[no_mangle]
pub extern "C" fn entry() -> ! {
    // SAFETY: firmware wrote a valid &dyn Platform fat-pointer (2 words) to
    // PLATFORM_PTR_ADDR before jumping here.
    let platform: &dyn Platform = unsafe { &**(PLATFORM_PTR_ADDR as *const *const dyn Platform) };
    common::boot_main(platform)
}

// Required by the xtensa-esp32-espidf std runtime (ESP-IDF app_main bridge).
// In QEMU mode the kernel binary is payload only — this is never called.
fn main() {
    entry()
}
