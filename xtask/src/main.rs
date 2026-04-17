use std::process::{Command, ExitStatus};
use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand};

// ─── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "xtask", about = "Flashpoint build orchestration")]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Build boot-rom and package it as flashpoint.rom
    BuildRom {
        /// Target platform: esp32 | esp32-s3 | rp2040
        #[arg(long, default_value = "esp32")]
        platform: String,

        /// Output ROM version X.Y.Z
        #[arg(long, default_value = "0.1.0")]
        version: String,

        /// Comma-separated required features e.g. psram,wifi
        #[arg(long)]
        requires: Option<String>,

        /// Output path for flashpoint.rom
        #[arg(long, default_value = "flashpoint.rom")]
        output: PathBuf,
    },

    /// Build the full flash-rom image (stage1 + optional embedded boot-rom)
    BuildFlash {
        /// Board target: esp32-cyd | esp32s3-xteink
        #[arg(long, default_value = "esp32-cyd")]
        target: String,

        /// Embed a boot-rom into the flash image
        #[arg(long)]
        embed_bootrom: bool,

        /// Path to flashpoint.rom to embed (required when --embed-bootrom)
        #[arg(long)]
        bootrom: Option<PathBuf>,
    },

    /// Flash the device via espflash
    Flash {
        /// Serial port e.g. /dev/ttyUSB0
        #[arg(long)]
        port: String,

        /// Board target: esp32-cyd | esp32s3-xteink
        #[arg(long, default_value = "esp32-cyd")]
        target: String,

        /// Embed a boot-rom into the flash image before flashing
        #[arg(long)]
        embed_bootrom: bool,
    },
}

// ─── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Task::BuildRom { platform, version, requires, output } =>
            cmd_build_rom(&platform, &version, requires.as_deref(), &output),
        Task::BuildFlash { target, embed_bootrom, bootrom } =>
            cmd_build_flash(&target, embed_bootrom, bootrom.as_deref()),
        Task::Flash { port, target, embed_bootrom } =>
            cmd_flash(&port, &target, embed_bootrom),
    };
    if let Err(e) = result {
        eprintln!("xtask error: {e}");
        std::process::exit(1);
    }
}

// ─── build-rom ───────────────────────────────────────────────────────────────

fn cmd_build_rom(
    platform: &str,
    version: &str,
    requires: Option<&str>,
    output: &Path,
) -> Result<(), String> {
    let target = platform_to_cargo_target(platform)?;

    println!("==> compiling boot-rom for {target}");
    run(
        Command::new("cargo")
            .args(["build", "-p", "boot-rom", "--target", target, "--release"])
    )?;

    let bin = workspace_root()
        .join("target")
        .join(target)
        .join("release")
        .join("boot-rom");

    println!("==> packaging {}", bin.display());
    let mut mkrom = Command::new("cargo");
    mkrom.args(["run", "-p", "tools", "--", "pack",
        "--platform", platform,
        "--version", version,
    ]);
    if let Some(r) = requires {
        mkrom.args(["--requires", r]);
    }
    mkrom.arg(bin.to_str().unwrap()).arg(output.to_str().unwrap());
    run(&mut mkrom)?;

    println!("==> {}", output.display());
    Ok(())
}

// ─── build-flash ─────────────────────────────────────────────────────────────

fn cmd_build_flash(
    target_board: &str,
    embed_bootrom: bool,
    bootrom_path: Option<&Path>,
) -> Result<(), String> {
    let esp_target = board_to_cargo_target(target_board)?;

    if embed_bootrom {
        let rom = match bootrom_path {
            Some(p) => p.to_path_buf(),
            None => {
                // build it first with defaults
                let out = PathBuf::from("flashpoint.rom");
                let platform = board_to_platform(target_board);
                cmd_build_rom(platform, "0.1.0", None, &out)?;
                out
            }
        };
        let rom_str = rom.to_str().unwrap();
        println!("==> compiling flash-rom (embed-bootrom) for {esp_target}");
        run(
            Command::new("cargo")
                .args(["build", "-p", "flash-rom", "--target", esp_target, "--release"])
                .env("BOOTROM_BIN", rom_str)
        )?;
    } else {
        println!("==> compiling flash-rom for {esp_target}");
        run(
            Command::new("cargo")
                .args(["build", "-p", "flash-rom", "--target", esp_target, "--release"])
        )?;
    }

    println!("==> flash-rom built for {target_board}");
    Ok(())
}

// ─── flash ───────────────────────────────────────────────────────────────────

fn cmd_flash(port: &str, target_board: &str, embed_bootrom: bool) -> Result<(), String> {
    cmd_build_flash(target_board, embed_bootrom, None)?;

    let esp_target = board_to_cargo_target(target_board)?;
    let bin = workspace_root()
        .join("target")
        .join(esp_target)
        .join("release")
        .join("flash-rom");

    println!("==> flashing {} to {port}", bin.display());
    run(
        Command::new("espflash")
            .args(["flash", "--port", port, bin.to_str().unwrap()])
    )?;
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn run(cmd: &mut Command) -> Result<(), String> {
    let status: ExitStatus = cmd
        .status()
        .map_err(|e| format!("failed to run {:?}: {e}", cmd.get_program()))?;
    if !status.success() {
        return Err(format!("{:?} exited with {status}", cmd.get_program()));
    }
    Ok(())
}

fn workspace_root() -> PathBuf {
    // xtask lives at <root>/xtask — parent is the workspace root
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn platform_to_cargo_target(platform: &str) -> Result<&'static str, String> {
    match platform.to_lowercase().as_str() {
        "esp32"              => Ok("xtensa-esp32-espidf"),
        "esp32-s3"|"esp32s3" => Ok("xtensa-esp32s3-espidf"),
        "rp2040"             => Ok("thumbv6m-none-eabi"),
        other => Err(format!("unknown platform '{other}'")),
    }
}

fn board_to_cargo_target(board: &str) -> Result<&'static str, String> {
    match board {
        "esp32-cyd"      => Ok("xtensa-esp32-espidf"),
        "esp32s3-xteink" => Ok("xtensa-esp32s3-espidf"),
        other => Err(format!("unknown board '{other}'")),
    }
}

fn board_to_platform(board: &str) -> &'static str {
    match board {
        "esp32-cyd"      => "esp32",
        "esp32s3-xteink" => "esp32-s3",
        _                => "esp32",
    }
}
