fn main() {
    const STAGE1_END: u32  = 0x10000; // 64 KB reserved for Stage 1 binary
    const FLASH_ALIGN: u32 = 0x1000;  // 4 KB sector alignment

    println!("cargo:rerun-if-env-changed=BOOTROM_BIN");

    match std::env::var("BOOTROM_BIN") {
        Ok(path) => {
            println!("cargo:rerun-if-changed={path}");
            let size = std::fs::metadata(&path)
                .unwrap_or_else(|e| panic!("BOOTROM_BIN='{}' not readable: {e}", path))
                .len() as u32;
            let aligned = align_up(size, FLASH_ALIGN);
            println!("cargo:rustc-env=BOOTROM_OFFSET={STAGE1_END}");
            println!("cargo:rustc-env=BOOTROM_SIZE={aligned}");
            println!("cargo:rustc-env=NVS_OFFSET={}", STAGE1_END + aligned);
        }
        Err(_) => {
            println!("cargo:rustc-env=BOOTROM_OFFSET=0");
            println!("cargo:rustc-env=BOOTROM_SIZE=0");
            println!("cargo:rustc-env=NVS_OFFSET={STAGE1_END}");
        }
    }
}

fn align_up(val: u32, align: u32) -> u32 {
    (val + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_up_already_aligned() {
        assert_eq!(align_up(0x1000, 0x1000), 0x1000);
        assert_eq!(align_up(0x2000, 0x1000), 0x2000);
    }

    #[test]
    fn align_up_rounds_up() {
        assert_eq!(align_up(0x1001, 0x1000), 0x2000);
        assert_eq!(align_up(0x1FFF, 0x1000), 0x2000);
        assert_eq!(align_up(1, 0x1000), 0x1000);
    }

    #[test]
    fn align_up_zero() {
        assert_eq!(align_up(0, 0x1000), 0);
    }
}
