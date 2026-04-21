use common::PlatformError;
use std::vec::Vec;

pub fn sd_read_sectors(_: u32, _: &mut [u8]) -> Result<(), PlatformError> {
    Err(PlatformError::SdReadError)
}
pub fn sd_write_sectors(_: u32, _: &[u8]) -> Result<(), PlatformError> {
    Err(PlatformError::SdWriteError)
}
pub fn sd_sector_count() -> u32 {
    0
}
pub fn nvs_read(_: &str, _: &str) -> Result<Vec<u8>, PlatformError> {
    Err(PlatformError::NvsError)
}
pub fn nvs_write(_: &str, _: &str, _: &[u8]) -> Result<(), PlatformError> {
    Err(PlatformError::NvsError)
}
pub fn nvs_delete(_: &str, _: &str) -> Result<(), PlatformError> {
    Err(PlatformError::NvsError)
}
