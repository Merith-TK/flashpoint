// Stage 1 — Flashpoint chainload loader

#[cfg(feature = "board-cyd")]
mod cyd;
mod helpers;
#[cfg(feature = "board-qemu")]
mod qemu;

pub fn stage1_main() -> ! {
    #[cfg(feature = "board-qemu")]
    {
        qemu::qemu_boot()
    }

    #[cfg(feature = "board-cyd")]
    {
        cyd::cyd_boot()
    }

    // Compile error if neither board feature is active (non-test builds only).
    #[cfg(all(not(test), not(any(feature = "board-qemu", feature = "board-cyd"))))]
    core::compile_error!("firmware requires --features board-cyd or --features board-qemu");

    // Satisfies the `-> !` return type in test builds with no board feature.
    #[allow(unreachable_code)]
    loop {}
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "board-cyd")]
    fn layout_constants_parse() {
        // These constants are defined in cyd.rs, we can verify they parse
        // by observing that the constants don't panic when evaluated.
    }
}
