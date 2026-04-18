fn main() {
    embuild::espidf::sysenv::output();

    println!("cargo:rerun-if-env-changed=FLASHPOINT_ROM");

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("flashpoint.rom");

    if let Ok(rom_path) = std::env::var("FLASHPOINT_ROM") {
        println!("cargo:rerun-if-changed={rom_path}");
        std::fs::copy(&rom_path, &dest).expect("failed to copy flashpoint.rom");
    } else {
        // No ROM set — write empty placeholder so include_bytes! always compiles.
        // validate_header will fail with TooShort, which is the expected behavior.
        std::fs::write(&dest, &[]).expect("failed to write placeholder rom");
    }

    println!("cargo:rustc-env=FLASHPOINT_ROM_PATH={}", dest.display());
}
