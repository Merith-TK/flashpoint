# flashpoint project environment — source this before running any build/flash commands
# safe to source multiple times

[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/scripts/export-esp.sh" ] && source "$SCRIPT_DIR/scripts/export-esp.sh"

export ESP_IDF_VERSION="v5.3.2"
export ESP_IDF_SYS_ROOT_CRATE="firmware"

# libxml2 SONAME compat: bundled esp-clang needs .so.2, Arch ships .so.16
export LD_LIBRARY_PATH="$HOME/.local/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
