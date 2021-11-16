//! Storj DCS Encryption key

use uplink_sys as ulksys;

/// TODO: implement & document it
pub struct EncryptionKey {
    inner: ulksys::UplinkEncryptionKeyResult,
}

impl EncryptionKey {
    pub(crate) fn as_uplink_c(&self) -> *mut ulksys::UplinkEncryptionKey {
        self.inner.encryption_key
    }
}
