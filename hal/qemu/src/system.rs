use common::{ChipId, FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING};

pub fn battery_percent() -> u8 {
    100
}
pub fn chip_id() -> ChipId {
    ChipId::Esp32
}
pub fn sleep_ms(ms: u32) {
    use esp_idf_svc::hal::delay::FreeRtos;
    FreeRtos::delay_ms(ms);
}
pub fn reboot() -> ! {
    panic!("reboot requested in emulator");
}
pub fn flashpoint_version() -> (u32, u32) {
    (FLASHPOINT_CURRENT, FLASHPOINT_LAST_BREAKING)
}
pub fn wasm_arena_limit() -> usize {
    256 * 1024
}
pub fn lua_heap_limit() -> usize {
    64 * 1024
}
