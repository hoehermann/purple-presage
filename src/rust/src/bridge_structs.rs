/* automatically generated by rust-bindgen 0.66.1 */

pub type PurpleAccount = [u64; 22usize];
#[allow(dead_code)]
pub const PURPLE_CONNECTION_ERROR_NETWORK_ERROR: PurpleConnectionError = 0;
#[allow(dead_code)]
pub const PURPLE_CONNECTION_ERROR_INVALID_USERNAME: PurpleConnectionError = 1;
pub const PURPLE_CONNECTION_ERROR_AUTHENTICATION_FAILED: PurpleConnectionError = 2;
/*
pub const PURPLE_CONNECTION_ERROR_AUTHENTICATION_IMPOSSIBLE:
    PurpleConnectionError = 3;
pub const PURPLE_CONNECTION_ERROR_NO_SSL_SUPPORT: PurpleConnectionError = 4;
pub const PURPLE_CONNECTION_ERROR_ENCRYPTION_ERROR: PurpleConnectionError = 5;
pub const PURPLE_CONNECTION_ERROR_NAME_IN_USE: PurpleConnectionError = 6;
pub const PURPLE_CONNECTION_ERROR_INVALID_SETTINGS: PurpleConnectionError = 7;
pub const PURPLE_CONNECTION_ERROR_CERT_NOT_PROVIDED: PurpleConnectionError =
    8;
pub const PURPLE_CONNECTION_ERROR_CERT_UNTRUSTED: PurpleConnectionError = 9;
pub const PURPLE_CONNECTION_ERROR_CERT_EXPIRED: PurpleConnectionError = 10;
pub const PURPLE_CONNECTION_ERROR_CERT_NOT_ACTIVATED: PurpleConnectionError =
    11;
pub const PURPLE_CONNECTION_ERROR_CERT_HOSTNAME_MISMATCH:
    PurpleConnectionError = 12;
pub const PURPLE_CONNECTION_ERROR_CERT_FINGERPRINT_MISMATCH:
    PurpleConnectionError = 13;
pub const PURPLE_CONNECTION_ERROR_CERT_SELF_SIGNED: PurpleConnectionError =
    14;
pub const PURPLE_CONNECTION_ERROR_CERT_OTHER_ERROR: PurpleConnectionError =
    15;
 */
pub const PURPLE_CONNECTION_ERROR_OTHER_ERROR: PurpleConnectionError = 16;
pub type PurpleConnectionError = ::std::os::raw::c_int;
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_SEND: PurpleMessageFlags = PurpleMessageFlags(1);
}
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_RECV: PurpleMessageFlags = PurpleMessageFlags(2);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_SYSTEM: PurpleMessageFlags = PurpleMessageFlags(4);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_AUTO_RESP: PurpleMessageFlags = PurpleMessageFlags(8);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_ACTIVE_ONLY: PurpleMessageFlags = PurpleMessageFlags(16);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_NICK: PurpleMessageFlags = PurpleMessageFlags(32);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_NO_LOG: PurpleMessageFlags = PurpleMessageFlags(64);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_WHISPER: PurpleMessageFlags = PurpleMessageFlags(128);
}
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_ERROR: PurpleMessageFlags = PurpleMessageFlags(512);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_DELAYED: PurpleMessageFlags = PurpleMessageFlags(1024);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_RAW: PurpleMessageFlags = PurpleMessageFlags(2048);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_IMAGES: PurpleMessageFlags = PurpleMessageFlags(4096);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_NOTIFY: PurpleMessageFlags = PurpleMessageFlags(8192);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_NO_LINKIFY: PurpleMessageFlags = PurpleMessageFlags(16384);
}
#[allow(dead_code)]
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_INVISIBLE: PurpleMessageFlags = PurpleMessageFlags(32768);
}
impl PurpleMessageFlags {
    pub const PURPLE_MESSAGE_REMOTE_SEND: PurpleMessageFlags = PurpleMessageFlags(65536);
}
impl ::std::ops::BitOr<PurpleMessageFlags> for PurpleMessageFlags {
    type Output = Self;
    #[inline]
    fn bitor(
        self,
        other: Self,
    ) -> Self {
        PurpleMessageFlags(self.0 | other.0)
    }
}
impl ::std::ops::BitOrAssign for PurpleMessageFlags {
    #[inline]
    fn bitor_assign(
        &mut self,
        rhs: PurpleMessageFlags,
    ) {
        self.0 |= rhs.0;
    }
}
impl ::std::ops::BitAnd<PurpleMessageFlags> for PurpleMessageFlags {
    type Output = Self;
    #[inline]
    fn bitand(
        self,
        other: Self,
    ) -> Self {
        PurpleMessageFlags(self.0 & other.0)
    }
}
impl ::std::ops::BitAndAssign for PurpleMessageFlags {
    #[inline]
    fn bitand_assign(
        &mut self,
        rhs: PurpleMessageFlags,
    ) {
        self.0 &= rhs.0;
    }
}
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct PurpleMessageFlags(pub ::std::os::raw::c_uint);
pub type PurpleXfer = [u64; 29usize];
pub type PurpleRoomlist = [u64; 7usize];
#[allow(dead_code)]
pub const PURPLE_DEBUG_ALL: PurpleDebugLevel = 0;
#[allow(dead_code)]
pub const PURPLE_DEBUG_MISC: PurpleDebugLevel = 1;
pub const PURPLE_DEBUG_INFO: PurpleDebugLevel = 2;
pub const PURPLE_DEBUG_WARNING: PurpleDebugLevel = 3;
pub const PURPLE_DEBUG_ERROR: PurpleDebugLevel = 4;
#[allow(dead_code)]
pub const PURPLE_DEBUG_FATAL: PurpleDebugLevel = 5;
pub type PurpleDebugLevel = ::std::os::raw::c_int;
pub type RustChannelPtr = *mut std::os::raw::c_void;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Group {
    pub key: *mut ::std::os::raw::c_char,
    pub title: *mut ::std::os::raw::c_char,
    pub description: *mut ::std::os::raw::c_char,
    pub revision: u32,
    pub members: *mut *mut ::std::os::raw::c_char,
    pub population: usize,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Message {
    pub account: *mut PurpleAccount,
    pub tx_ptr: RustChannelPtr,
    pub qrcode: *mut ::std::os::raw::c_char,
    pub uuid: *mut ::std::os::raw::c_char,
    pub debug: PurpleDebugLevel,
    pub error: PurpleConnectionError,
    pub connected: i32,
    pub padding: i32,
    pub timestamp: u64,
    pub flags: PurpleMessageFlags,
    pub who: *mut ::std::os::raw::c_char,
    pub name: *mut ::std::os::raw::c_char,
    pub phone_number: *mut ::std::os::raw::c_char,
    pub group: *mut ::std::os::raw::c_char,
    pub body: *mut ::std::os::raw::c_char,
    pub blob: *mut ::std::os::raw::c_void,
    pub size: usize,
    pub groups: *mut Group,
    pub roomlist: *mut PurpleRoomlist,
    pub xfer: *mut PurpleXfer,
}
