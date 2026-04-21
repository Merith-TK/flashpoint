#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderError {
    TooShort,
    BadMagic,
    WrongPlatform,
    ApiIncompatible,
    BadTerminator,
    UnsupportedHeaderVersion,
    MissingFeatures,
    BadPayloadLen,
    BadChecksum,
    UnknownPayloadType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformError {
    SdReadError,
    SdWriteError,
    NvsError,
    DisplayError,
    NotSupported,
}
