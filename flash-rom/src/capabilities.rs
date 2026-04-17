use flashpoint_common::*;

// DEVICE_FEATURES declares what hardware this flash-rom build provides.
// Each board target gets its own set of flags.
//
// CYD (ESP32-2432S028R):
//   - ILI9341 TFT display  → FEAT_DISPLAY_TFT
//   - XPT2046 touch panel  → FEAT_INPUT_TOUCH
//   - No PSRAM, no WiFi (base flash-rom; WiFi extension is Phase 4)
//   - No battery monitoring
//
// To change the target board, adjust the flags below or use cfg features.

pub const DEVICE_FEATURES: u64 =
    FEAT_DISPLAY_TFT |  // ILI9341 via SPI
    FEAT_INPUT_TOUCH;   // XPT2046 resistive touch
