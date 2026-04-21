use common::PlatformError;
use esp_idf_svc::sys as idf;
use std::ffi::CString;
use std::vec::Vec;

// ─── NVS helpers (raw esp-idf-sys C bindings) ────────────────────────────────

pub(crate) fn nvs_get(ns: &str, key: &str) -> Result<Vec<u8>, PlatformError> {
    unsafe {
        let ns_c = CString::new(ns).map_err(|_| PlatformError::NvsError)?;
        let key_c = CString::new(key).map_err(|_| PlatformError::NvsError)?;

        let mut handle: idf::nvs_handle_t = 0;
        idf::nvs_open(
            ns_c.as_ptr(),
            idf::nvs_open_mode_t_NVS_READONLY,
            &mut handle,
        );

        let mut size: usize = 0;
        let rc = idf::nvs_get_blob(handle, key_c.as_ptr(), core::ptr::null_mut(), &mut size);
        if rc != idf::ESP_OK {
            idf::nvs_close(handle);
            return Err(PlatformError::NvsError);
        }

        let mut buf = vec![0u8; size];
        let rc = idf::nvs_get_blob(
            handle,
            key_c.as_ptr(),
            buf.as_mut_ptr() as *mut _,
            &mut size,
        );
        idf::nvs_close(handle);

        if rc == idf::ESP_OK {
            Ok(buf)
        } else {
            Err(PlatformError::NvsError)
        }
    }
}

pub(crate) fn nvs_set(ns: &str, key: &str, val: &[u8]) -> Result<(), PlatformError> {
    unsafe {
        let ns_c = CString::new(ns).map_err(|_| PlatformError::NvsError)?;
        let key_c = CString::new(key).map_err(|_| PlatformError::NvsError)?;

        let mut handle: idf::nvs_handle_t = 0;
        let rc = idf::nvs_open(
            ns_c.as_ptr(),
            idf::nvs_open_mode_t_NVS_READWRITE,
            &mut handle,
        );
        if rc != idf::ESP_OK {
            return Err(PlatformError::NvsError);
        }

        let rc = idf::nvs_set_blob(handle, key_c.as_ptr(), val.as_ptr() as *const _, val.len());
        if rc == idf::ESP_OK {
            idf::nvs_commit(handle);
        }
        idf::nvs_close(handle);

        if rc == idf::ESP_OK {
            Ok(())
        } else {
            Err(PlatformError::NvsError)
        }
    }
}

pub(crate) fn nvs_erase(ns: &str, key: &str) -> Result<(), PlatformError> {
    unsafe {
        let ns_c = CString::new(ns).map_err(|_| PlatformError::NvsError)?;
        let key_c = CString::new(key).map_err(|_| PlatformError::NvsError)?;

        let mut handle: idf::nvs_handle_t = 0;
        let rc = idf::nvs_open(
            ns_c.as_ptr(),
            idf::nvs_open_mode_t_NVS_READWRITE,
            &mut handle,
        );
        if rc != idf::ESP_OK {
            return Err(PlatformError::NvsError);
        }

        let rc = idf::nvs_erase_key(handle, key_c.as_ptr());
        if rc == idf::ESP_OK {
            idf::nvs_commit(handle);
        }
        idf::nvs_close(handle);

        if rc == idf::ESP_OK {
            Ok(())
        } else {
            Err(PlatformError::NvsError)
        }
    }
}
