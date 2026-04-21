use common::{FrameBuffer, PlatformError};

pub fn clear() -> Result<(), PlatformError> {
    log::info!("[display] clear");
    Ok(())
}

pub fn flush(buf: &FrameBuffer) -> Result<(), PlatformError> {
    if buf.y % 60 == 0 {
        log::info!("[display] scanline y={}", buf.y);
    }
    Ok(())
}

pub fn width() -> u16 {
    320
}
pub fn height() -> u16 {
    240
}
