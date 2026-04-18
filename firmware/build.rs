// flash-rom/build.rs
//
// Forwards BOOTROM_BIN to the stage1 build so its build.rs can
// calculate BOOTROM_OFFSET / BOOTROM_SIZE / NVS_OFFSET correctly.
// The xtask sets BOOTROM_BIN before invoking cargo for this crate.

fn main() {
    println!("cargo:rerun-if-env-changed=BOOTROM_BIN");
    if let Ok(path) = std::env::var("BOOTROM_BIN") {
        println!("cargo:rerun-if-changed={path}");
        // Confirm the file exists so we fail early rather than silently
        if !std::path::Path::new(&path).exists() {
            panic!("BOOTROM_BIN='{}' does not exist", path);
        }
    }
}
