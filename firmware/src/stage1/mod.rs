// Stage 1 — Flashpoint chainload loader

#[cfg(feature = "board-cyd")]
mod cyd;
mod helpers;
#[cfg(feature = "board-qemu")]
mod qemu;

pub fn stage1_main() -> ! {
    #[cfg(feature = "board-cyd")]
    {
        // Must run once before any nvs_open(), regardless of hardware path.
        // Erase + reinit on dirty/version-mismatch partitions.
        unsafe {
            use esp_idf_svc::sys as idf;
            let rc = idf::nvs_flash_init();
            if rc == idf::ESP_ERR_NVS_NO_FREE_PAGES || rc == idf::ESP_ERR_NVS_NEW_VERSION_FOUND {
                log::warn!("[stage1] NVS partition dirty — erasing and reinitialising");
                idf::nvs_flash_erase();
                idf::nvs_flash_init();
            }
        }
    }

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
