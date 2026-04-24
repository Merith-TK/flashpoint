use common::PlatformError;
use std::vec::Vec;

// NVS is no longer backed by ESP-IDF NVS flash.
// All nvs_* calls via CydPlatform return Err — the firmware layer
// wraps CydPlatform in SdPlatform which redirects to SD-backed tinykv stores.

pub(crate) fn nvs_get(_ns: &str, _key: &str) -> Result<Vec<u8>, PlatformError> {
    Err(PlatformError::NvsError)
}

pub(crate) fn nvs_set(_ns: &str, _key: &str, _val: &[u8]) -> Result<(), PlatformError> {
    Err(PlatformError::NvsError)
}

pub(crate) fn nvs_erase(_ns: &str, _key: &str) -> Result<(), PlatformError> {
    Err(PlatformError::NvsError)
}
