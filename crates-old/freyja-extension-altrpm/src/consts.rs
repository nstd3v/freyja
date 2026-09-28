use std::time::Duration;

pub const CACHE_TTL: Duration = Duration::from_secs(60 * 60);

pub const RPM_HEADER_MAGIC: [u8; 3] = [0x8e, 0xad, 0xe8];

// RPM header tags we care about.
pub const RPMTAG_NAME: u32 = 1000;
pub const RPMTAG_VERSION: u32 = 1001;
pub const RPMTAG_RELEASE: u32 = 1002;
pub const RPMTAG_EPOCH: u32 = 1003;
pub const RPMTAG_ARCH: u32 = 1022;

// RPM header data types.
pub const RPM_INT32_TYPE: u32 = 4;
pub const RPM_STRING_TYPE: u32 = 6;
