//! Storj DCS linksharing service operations and related types.

use crate::uplink_c::{string_from_ffi_string_result, Ensurer};
use crate::{helpers, Result};

use std::ffi::CString;
use std::ptr;

use uplink_sys as ulksys;

/// It doesn't check the existence or the accessibility of the target.
///
/// An example result is
/// https://link.us1.storjshare.io/s/l5pucy3dmvzxgs3fpfewix27l5pq/mybucket/myprefix/myobject
///
/// * The `base_url` is the URL of the linksharing service, e.g. https://link.us1.storjshare.io.
/// * The `access_key_id` is can be obtained calling [`crate::edge::Config::register_gateway_access`]
///   but, it must be associated with public visibility.
/// * The `bucket` is the name of the bucket to share; set it to empty for sharing the entire
///   project.
/// * The `key` is the object key to share; set it to empty for sharing the entire bucket. It
///   accepts to be a prefix but then it has to end with `/`.
pub fn share_url<'a>(
    base_url: &str,
    access_key_id: &str,
    bucket: &str,
    key: &str,
    opts: Option<OptionsShareURL>,
) -> Result<&'a str> {
    let base_url = helpers::cstring_from_str_fn_arg("base_url", base_url)?;
    let access_key = helpers::cstring_from_str_fn_arg("access_key_id", access_key_id)?;
    let bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
    let key = helpers::cstring_from_str_fn_arg("key", key)?;
    let res: ulksys::UplinkStringResult;

    unsafe {
        let c_base_url = base_url.into_raw();
        let c_access_key = access_key.into_raw();
        let c_bucket = bucket.into_raw();
        let c_key = key.into_raw();

        if let Some(o) = opts {
            let edge_opts = o.as_ffi_options_share_url();
            let c_opts = &edge_opts as *const ulksys::EdgeShareURLOptions;
            res = ulksys::edge_join_share_url(
                c_base_url,
                c_access_key,
                c_bucket,
                c_key,
                c_opts as *mut ulksys::EdgeShareURLOptions,
            );
        } else {
            res = ulksys::edge_join_share_url(
                c_base_url,
                c_access_key,
                c_bucket,
                c_key,
                ptr::null_mut(),
            );
        }

        // Retake ownership from C for dropping them and not leaking memory.
        let _ = CString::from_raw(c_base_url);
        let _ = CString::from_raw(c_access_key);
        let _ = CString::from_raw(c_bucket);
        let _ = CString::from_raw(c_key);
    }

    res.ensure();
    string_from_ffi_string_result(res)
}

/// Contains the options parameters for creating the share URL.
/// See [`share_url` function](share_url).
pub struct OptionsShareURL {
    /// `true` indicates to create a direct URL to the data, otherwise to an intermediate landing
    /// page.
    ///
    /// The direct URL to the data works for downloading data with some command or embedding it on
    /// a web page.
    pub raw: bool,
}

impl OptionsShareURL {
    /// Returns the FFI representation of this options.
    pub(crate) fn as_ffi_options_share_url(&self) -> ulksys::EdgeShareURLOptions {
        ulksys::EdgeShareURLOptions { raw: self.raw }
    }
}
