use crate::constants::*;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

pub fn parse_features(s: &str) -> Result<u64, &str> {
    let mut bits = 0u64;
    for part in s.split(',') {
        bits |= match part.trim() {
            "wifi" => FEAT_WIFI,
            "ble" => FEAT_BLE,
            "usb_otg" => FEAT_USB_OTG,
            "disp_tft" => FEAT_DISP_TFT,
            "disp_eink" => FEAT_DISP_EINK,
            "input_touch" => FEAT_INPUT_TOUCH,
            "input_buttons" => FEAT_INPUT_BUTTONS,
            "psram" => FEAT_PSRAM,
            "battery" => FEAT_BATTERY,
            other => return Err(other),
        };
    }
    Ok(bits)
}

#[cfg(feature = "std")]
pub fn features_to_names(bits: u64) -> std::vec::Vec<&'static str> {
    let mut names = std::vec::Vec::new();
    if bits & FEAT_WIFI != 0 {
        names.push("wifi");
    }
    if bits & FEAT_BLE != 0 {
        names.push("ble");
    }
    if bits & FEAT_USB_OTG != 0 {
        names.push("usb_otg");
    }
    if bits & FEAT_DISP_TFT != 0 {
        names.push("disp_tft");
    }
    if bits & FEAT_DISP_EINK != 0 {
        names.push("disp_eink");
    }
    if bits & FEAT_INPUT_TOUCH != 0 {
        names.push("input_touch");
    }
    if bits & FEAT_INPUT_BUTTONS != 0 {
        names.push("input_buttons");
    }
    if bits & FEAT_PSRAM != 0 {
        names.push("psram");
    }
    if bits & FEAT_BATTERY != 0 {
        names.push("battery");
    }
    names
}
