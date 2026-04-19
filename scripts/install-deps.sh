#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
ENV_FILE="$PROJECT_ROOT/.env.sh"

echo "==> Installing system dependencies..."
yay -S --noconfirm \
    python \
    python-pip \
    python-pyserial \
    cmake \
    ninja \
    wget \
    unzip \
    libusb \
    dfu-util \
    esptool \
    qemu-system-xtensa

echo "==> Installing Rust via rustup..."
if ! command -v rustup &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        --default-toolchain stable \
        --profile minimal
    source "$HOME/.cargo/env"
else
    echo "    rustup already installed, skipping"
fi

export PATH="$HOME/.cargo/bin:$PATH"

echo "==> Installing Xtensa (ESP32) Rust toolchain via espup..."
if ! command -v espup &>/dev/null; then
    cargo install espup
fi
espup install --targets esp32,esp32s3

echo "==> Installing ESP32 cargo tools..."
cargo install espflash ldproxy

echo "==> Creating libxml2 SONAME compat symlink (esp-clang needs .so.2, Arch ships .so.16)..."
mkdir -p "$HOME/.local/lib"
ln -sf /usr/lib/libxml2.so.16 "$HOME/.local/lib/libxml2.so.2"

echo "==> Generating $ENV_FILE ..."
cat > "$ENV_FILE" << ENVEOF
# flashpoint project environment — source this before running any build/flash commands
# safe to source multiple times

[ -f "\$HOME/.cargo/env" ] && source "\$HOME/.cargo/env"
[ -f "$SCRIPT_DIR/export-esp.sh" ] && source "$SCRIPT_DIR/export-esp.sh"

export ESP_IDF_VERSION="v5.3.2"
export ESP_IDF_SYS_ROOT_CRATE="firmware"
ENVEOF
chmod +x "$ENV_FILE"

echo "==> Adding .env.sh source to ~/.bashrc ..."
BASHRC="$HOME/.bashrc"
BASHRC_LINE="[ -f \"$ENV_FILE\" ] && source \"$ENV_FILE\"  # flashpoint"
grep -qxF "$BASHRC_LINE" "$BASHRC" 2>/dev/null || echo "$BASHRC_LINE" >> "$BASHRC"

echo ""
echo "Done. Reload your shell or run: source ~/.bashrc"
