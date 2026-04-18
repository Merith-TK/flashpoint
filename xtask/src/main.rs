use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use clap::{Parser, Subcommand};

mod rom;

// ─── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "xtask", about = "Flashpoint build orchestration")]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Check that all required tools are installed
    Setup,

    /// Build the OS kernel and package it as flashpoint.rom (for SD card or embedding)
    BuildBoot {
        /// Target platform: esp32 | esp32-s3 | rp2040
        #[arg(long, default_value = "esp32")]
        platform: String,

        /// ROM version X.Y.Z
        #[arg(long, default_value = "0.1.0")]
        version: String,

        /// Flashpoint API version this ROM targets (default: current firmware version)
        #[arg(long)]
        built_against: Option<String>,

        /// Required hardware features e.g. psram,wifi
        #[arg(long)]
        requires: Option<String>,

        /// Output path for flashpoint.rom
        #[arg(long, default_value = "flashpoint.rom")]
        output: PathBuf,
    },

    /// Build the device firmware (Stage 1 + HAL drivers) burned to internal flash
    BuildFlash {
        /// Board target: esp32-cyd | esp32s3-xteink
        #[arg(long, default_value = "esp32-cyd")]
        board: String,

        /// Also build the kernel and embed it into the firmware image
        #[arg(long)]
        embed_boot: bool,

        /// Path to an existing flashpoint.rom to embed (skips build-boot when provided)
        #[arg(long)]
        bootrom: Option<PathBuf>,
    },

    /// Create a merged flash binary (bootloader + partition table + app) ready for espflash/QEMU
    BuildImage {
        /// Board target: esp32-cyd | esp32s3-xteink
        #[arg(long, default_value = "esp32-cyd")]
        board: String,

        /// Output path for the merged flash image
        #[arg(long, default_value = "flash.bin")]
        output: PathBuf,
    },

    /// Build the emulator binary and create a merged flash image for QEMU
    EmuBuild {
        /// Output path for the merged flash image
        #[arg(long, default_value = "emulator/flash.bin")]
        output: PathBuf,
    },

    /// Build the emulator and launch it in qemu-esp-xtensa
    EmuRun {
        /// Extra flags to pass to QEMU
        #[arg(last = true)]
        qemu_args: Vec<String>,
    },

    /// Build the firmware image and flash it to a connected device
    Flash {
        /// Serial port e.g. /dev/ttyUSB0
        #[arg(long)]
        port: String,

        /// Board target: esp32-cyd | esp32s3-xteink
        #[arg(long, default_value = "esp32-cyd")]
        board: String,

        /// Also embed the kernel before flashing
        #[arg(long)]
        embed_boot: bool,
    },

    /// Wrap a raw binary with a Flashpoint ROM header → flashpoint.rom
    Pack {
        #[arg(long)]
        platform: String,
        #[arg(long)]
        version: String,
        /// Flashpoint API version this ROM targets (default: current firmware version)
        #[arg(long)]
        built_against: Option<String>,
        #[arg(long)]
        requires: Option<String>,
        #[arg(long, default_value_t = false)]
        compress: bool,
        input: PathBuf,
        output: PathBuf,
    },

    /// Parse and validate a flashpoint.rom file
    Verify {
        input: PathBuf,
    },
}

// ─── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Task::Setup => cmd_setup(),
        Task::BuildBoot { platform, version, built_against, requires, output } =>
            cmd_build_boot(&platform, &version, built_against.as_deref(), requires.as_deref(), &output),
        Task::BuildFlash { board, embed_boot, bootrom } =>
            cmd_build_flash(&board, embed_boot, bootrom.as_deref()),
        Task::BuildImage { board, output } =>
            cmd_build_image(&board, &output),
        Task::EmuBuild { output } =>
            cmd_emu_build(&output),
        Task::EmuRun { qemu_args } =>
            cmd_emu_run(&qemu_args),
        Task::Flash { port, board, embed_boot } =>
            cmd_flash(&port, &board, embed_boot),
        Task::Pack { platform, version, built_against, requires, compress, input, output } =>
            rom::do_pack(&platform, &version, built_against.as_deref(), requires.as_deref(), compress, &input, &output),
        Task::Verify { input } =>
            rom::do_verify(&input),
    };
    if let Err(e) = result {
        eprintln!("xtask error: {e}");
        std::process::exit(1);
    }
}

// ─── setup ───────────────────────────────────────────────────────────────────

fn cmd_setup() -> Result<(), String> {
    println!("==> checking Flashpoint build dependencies");
    let mut ok = true;

    let tools = [
        ("cargo",              "Rust toolchain"),
        ("espflash",           "cargo install espflash"),
        ("ldproxy",            "cargo install ldproxy"),
        ("qemu-esp-xtensa",    "https://github.com/espressif/qemu/releases"),
    ];

    for (bin, hint) in &tools {
        if which(bin) {
            println!("  [✓] {bin}");
        } else {
            println!("  [✗] {bin}  →  {hint}");
            ok = false;
        }
    }

    // Check esp toolchain (rustup toolchain named "esp")
    let esp_ok = Command::new("cargo")
        .args(["+esp", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if esp_ok {
        println!("  [✓] cargo +esp  (Xtensa Rust toolchain)");
    } else {
        println!("  [✗] cargo +esp  →  run: espup install");
        ok = false;
    }

    // Check LIBCLANG_PATH (needed for esp-idf-sys bindgen)
    if std::env::var("LIBCLANG_PATH").is_ok() || detect_libclang().is_some() {
        println!("  [✓] LIBCLANG_PATH  (esp-clang for bindgen)");
    } else {
        println!("  [✗] LIBCLANG_PATH  →  run: source scripts/export-esp.sh");
        ok = false;
    }

    if ok {
        println!("\nAll dependencies satisfied. You're ready to build.");
    } else {
        println!("\nSome dependencies are missing. See hints above.");
        return Err("missing dependencies".into());
    }
    Ok(())
}

fn which(bin: &str) -> bool {
    Command::new("which").arg(bin).output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ─── build-boot ──────────────────────────────────────────────────────────────

fn cmd_build_boot(
    platform: &str,
    version: &str,
    built_against: Option<&str>,
    requires: Option<&str>,
    output: &Path,
) -> Result<(), String> {
    let target = platform_to_target(platform)?;

    println!("==> compiling kernel for {target}");
    run(esp_cmd("cargo")
        .args(["build", "-p", "kernel", "--target", target, "--release"]))?;

    let bin = workspace_root()
        .join("target").join(target).join("release").join("kernel");

    println!("==> packaging {} → {}", bin.display(), output.display());
    rom::do_pack(platform, version, built_against, requires, false, &bin, output)
}

// ─── build-flash ─────────────────────────────────────────────────────────────

fn cmd_build_flash(
    board: &str,
    embed_boot: bool,
    bootrom_path: Option<&Path>,
) -> Result<(), String> {
    let target = board_to_target(board)?;

    let mut cmd = esp_cmd("cargo");
    cmd.args(["build", "-p", "firmware", "--target", target, "--release"]);

    if embed_boot {
        let rom = match bootrom_path {
            Some(p) => p.to_path_buf(),
            None => {
                let out = PathBuf::from("flashpoint.rom");
                cmd_build_boot(board_to_platform(board), "0.1.0", None, None, &out)?;
                out
            }
        };
        println!("==> compiling firmware (embed-boot: {}) for {target}", rom.display());
        cmd.env("BOOTROM_BIN", rom.to_str().unwrap());
    } else {
        println!("==> compiling firmware for {target}");
    }

    run(&mut cmd)
}

// ─── build-image ─────────────────────────────────────────────────────────────

fn cmd_build_image(board: &str, output: &Path) -> Result<(), String> {
    let target = board_to_target(board)?;
    cmd_build_flash(board, false, None)?;

    let bin = workspace_root()
        .join("target").join(target).join("release").join("firmware");

    println!("==> creating merged flash image → {}", output.display());
    run(Command::new("espflash")
        .args(["save-image", "--chip", board_to_chip(board), "--merge",
            bin.to_str().unwrap(),
            output.to_str().unwrap(),
        ]))
}

// ─── emu-build ───────────────────────────────────────────────────────────────

fn cmd_emu_build(output: &Path) -> Result<(), String> {
    // Step 1: build kernel → flashpoint.rom (embedded into the emulator binary)
    let rom = workspace_root().join("flashpoint.rom");
    cmd_build_boot("esp32", "0.1.0", None, None, &rom)?;

    // Step 2: compile emulator with ROM embedded at compile time
    println!("==> compiling emulator (FLASHPOINT_ROM={})", rom.display());
    run(esp_cmd("cargo")
        .args(["build", "-p", "emulator", "--release"])
        .env("FLASHPOINT_ROM", rom.to_str().unwrap()))?;

    let bin = workspace_root()
        .join("target/xtensa-esp32-espidf/release/emulator");

    // Step 3: merge into a single flash image for QEMU
    println!("==> creating merged flash image → {}", output.display());
    run(Command::new("espflash")
        .args(["save-image", "--chip", "esp32", "--merge",
            bin.to_str().unwrap(),
            output.to_str().unwrap(),
        ]))
}

// ─── emu-run ─────────────────────────────────────────────────────────────────

fn cmd_emu_run(extra: &[String]) -> Result<(), String> {
    let flash_img = PathBuf::from("emulator/flash.bin");
    cmd_emu_build(&flash_img)?;

    println!("==> launching qemu-esp-xtensa");
    let mut cmd = Command::new("qemu-esp-xtensa");
    cmd.args([
        "-nographic",
        "-machine", "esp32",
        "-drive", &format!("if=mtd,format=raw,file={}", flash_img.display()),
    ]);
    for arg in extra {
        cmd.arg(arg);
    }
    run(&mut cmd)
}

// ─── flash ───────────────────────────────────────────────────────────────────

fn cmd_flash(port: &str, board: &str, embed_boot: bool) -> Result<(), String> {
    let img = PathBuf::from("flash.bin");
    let target = board_to_target(board)?;
    cmd_build_flash(board, embed_boot, None)?;

    let bin = workspace_root()
        .join("target").join(target).join("release").join("firmware");

    println!("==> creating merged flash image → {}", img.display());
    run(Command::new("espflash")
        .args(["save-image", "--chip", board_to_chip(board), "--merge",
            bin.to_str().unwrap(),
            img.to_str().unwrap(),
        ]))?;

    println!("==> flashing to {port}");
    run(Command::new("espflash")
        .args(["flash", "--port", port, img.to_str().unwrap()]))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn run(cmd: &mut Command) -> Result<(), String> {
    // Apply ESP toolchain env if not already set
    if std::env::var("LIBCLANG_PATH").is_err() {
        if let Some(libclang) = detect_libclang() {
            cmd.env("LIBCLANG_PATH", &libclang);
        }
    }
    if let Some(gcc_bin) = detect_gcc_bin() {
        let path = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("{}:{path}", gcc_bin.display()));
    }

    let status: ExitStatus = cmd
        .status()
        .map_err(|e| format!("failed to run {:?}: {e}", cmd.get_program()))?;
    if !status.success() {
        return Err(format!("{:?} exited with {status}", cmd.get_program()));
    }
    Ok(())
}

/// Spawn a cargo command using the `+esp` toolchain for Xtensa cross-compilation.
fn esp_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.arg("+esp");
    cmd
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn platform_to_target(platform: &str) -> Result<&'static str, String> {
    match platform.to_lowercase().as_str() {
        "esp32"               => Ok("xtensa-esp32-espidf"),
        "esp32-s3" | "esp32s3" => Ok("xtensa-esp32s3-espidf"),
        "rp2040"              => Ok("thumbv6m-none-eabi"),
        other => Err(format!("unknown platform '{other}': use esp32 | esp32-s3 | rp2040")),
    }
}

fn board_to_target(board: &str) -> Result<&'static str, String> {
    match board {
        "esp32-cyd"       => Ok("xtensa-esp32-espidf"),
        "esp32s3-xteink"  => Ok("xtensa-esp32s3-espidf"),
        other => Err(format!("unknown board '{other}': use esp32-cyd | esp32s3-xteink")),
    }
}

fn board_to_platform(board: &str) -> &'static str {
    match board {
        "esp32s3-xteink" => "esp32-s3",
        _                => "esp32",
    }
}

fn board_to_chip(board: &str) -> &'static str {
    match board {
        "esp32s3-xteink" => "esp32s3",
        _                => "esp32",
    }
}

/// Detect LIBCLANG_PATH from espup's toolchain install under ~/.rustup/toolchains/esp/
fn detect_libclang() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let base = PathBuf::from(home)
        .join(".rustup/toolchains/esp/xtensa-esp32-elf-clang");
    first_child_subpath(&base, "esp-clang/lib")
        .map(|p| p.to_string_lossy().into_owned())
}

/// Detect the Xtensa GCC bin dir from espup's toolchain install.
fn detect_gcc_bin() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let base = PathBuf::from(home)
        .join(".rustup/toolchains/esp/xtensa-esp-elf");
    first_child_subpath(&base, "xtensa-esp-elf/bin")
}

fn first_child_subpath(parent: &Path, suffix: &str) -> Option<PathBuf> {
    std::fs::read_dir(parent).ok()?
        .flatten()
        .map(|e| e.path().join(suffix))
        .find(|p| p.exists())
}
