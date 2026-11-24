//! Errors returned by this crate.

use std::error as stderr;
use std::ffi::CStr;
use std::fmt;

use uplink_sys as ulksys;

/// Convenient type alias to shorten the signature on every usage.
pub(crate) type BoxError = Box<dyn stderr::Error + Send + Sync>;

/// The error type that this crate use for wrapping errors.
#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    /// Identifies errors produced by the internal implementation (e.g. exchanging values with the
    /// C, etc. )that aren't expected to happen.
    Internal(Internal),
    /// Identifies invalid arguments passed to a function or method.
    InvalidArguments(Args),
    /// Identifies a native error returned by the FFI.
    Uplink(Uplink),
}

impl Error {
    /// Creates an [`Internal` variant](Self::Internal) from the provided context message and the
    /// error that originated it.
    pub(crate) fn new_internal(ctx_msg: &str, err: BoxError) -> Self {
        Error::Internal(Internal {
            ctx_msg: String::from(ctx_msg),
            inner: err,
        })
    }

    /// Convenient constructor for creating an [`InvalidArguments` variant](Self::InvalidArguments)
    /// Error.
    ///
    /// See [`Args`] documentation to know about the convention for the value of the `names`
    /// parameter because this constructor will panic in the future when the constraints will be
    /// implemented by [`Args::new`] constructor.
    pub(crate) fn new_invalid_arguments(names: &str, msg: &str) -> Self {
        Self::InvalidArguments(Args::new(names, msg))
    }

    /// Convenient constructor for creating an [`Uplink` variant](Self::Uplink).
    ///
    /// See [`Uplink::with_ffi_error`].
    pub(crate) fn with_ffi_error(err: &ulksys::UplinkError) -> Self {
        Self::Uplink(Uplink::with_ffi_error(err))
    }
}

impl stderr::Error for Error {
    fn source(&self) -> Option<&(dyn stderr::Error + 'static)> {
        match self {
            Error::InvalidArguments { .. } => None,
            Error::Uplink { .. } => None,
            Error::Internal(Internal { inner, .. }) => Some(inner.as_ref()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::InvalidArguments(args) => {
                write!(f, "{}", args)
            }
            Error::Uplink(details) => {
                write!(f, "{}", details)
            }
            Error::Internal(details) => {
                write!(f, "{}", details)
            }
        }
    }
}

/// Represents invalid arguments error regarding the business domain.
///
/// # Example
///
/// ```ignore
/// // This example is ignored because it shows how to return an `InvalidArguments` error through
/// // the constructor methods that aren't exported outside of this crate.
///
/// use storj_uplink_lib::{Error, Result};
///
/// fn positive_non_zero_div_and_mul(a: i64, b: i64, div: i64) ->Result<i64> {
///     if div == 0 {
///         return Err(Error::new_invalid_arguments("div", "div cannot be 0"));
///     }
///
///     if (a == 0 && b != 0) || (a != 0 && b == 0) {
///         return Err(Error::new_invalid_arguments(
///             "(a,b)", "a and b can only be 0 if both are 0",
///         ));
///     }
///
///     if (a >= 0 && b >= 0 && div > 0) || (a <= 0 && b <= 0 && div < 0 ) {
///         return Ok((a/div) * (b/div));
///     }
///
///     Err(Error::new_invalid_arguments(
///         "<all>", "all the arguments must be positive or negative, they cannot be mixed",
///     ))
/// }
/// ```
#[derive(Debug)]
pub struct Args {
    /// One or several parameters names; it has several conventions for expressing the involved
    /// parameters.
    ///
    /// * When a specific parameter is invalid its value is the exact parameter name.
    /// * When the parameter is a list (vector, array, etc.), the invalid items can be
    ///   __optionally__ indicated using square brackets (e.g. `l[3,5,7]`).
    /// * when the parameter is struct, the invalid fields or method return return values can be
    ///   __optionally__ indicated using curly brackets (e.g invalid field: `person{name}`, invalid
    ///   method return value: `person{full_name()}`, invalid fields/methods:
    ///   `employee{name, position()}`).
    /// * When several parameters are invalid, its values is the parameters names wrapped in round
    ///   brackets (e.g. `(p1,p3)`); it also accepts any above combination of parameters types
    ///   (e.g. `(p1, l[2,10], person{name})`).
    /// * When all the function parameters are invalid, `<all>` is used.
    ///
    // For enforcing these constrains internally use [`Self::new`].
    pub names: String,
    /// A human friendly message that explains why the argument(s) are invalid.
    pub msg: String,
}

impl Args {
    /// Creates a new instance or panic if [`Self::names`] field's constrains mentioned in the
    /// fields' documentation are violated.
    ///
    /// It panics because it makes easier to find a BUG on the `name`'s passed value.
    ///
    /// TODO(https://github.com/storj-thirdparty/uplink-rust/issues/52): Implement the `name`'s
    /// constraints validation and panic if the validation fails.
    fn new(names: &str, msg: &str) -> Self {
        Args {
            names: String::from(names),
            msg: String::from(msg),
        }
    }
}

impl fmt::Display for Args {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{} arguments have invalid values. {}",
            self.names, self.msg
        )
    }
}

/// Wraps a native error returned by the FFI providing the access to its details.
#[derive(Debug)]
pub enum Uplink {
    /// A Storj DCS network internal error.
    Internal(String),
    /// A Storj DCS network cancellation error.
    Canceled(String),
    /// An invalid handle is passed to the FFI. This error shouldn't happen and
    /// when it does, it's likely due to a bug in the FFI.
    InvalidHandle(String),
    /// Storj DCS network rejected the operation because the client over passed the rate-limit
    /// allowance.
    TooManyRequests(String),
    /// Storj DCS network rejected the operation because the bandwidth limit of the client has
    /// reached all the user account's bandwidth. User should upgrade its account to a another plan
    /// or they are not able then, reach Storj DCS support.
    BandwidthLimitExceeded(String),
    /// Storj DCS network rejected the operation because the bucket's name contains non-allowed
    /// characters.
    BucketNameInvalid(String),
    /// Storj DCS network rejected the operation because the bucket already exists.
    BucketAlreadyExists(String),
    /// Storj DCS network rejected the operation because the bucket still contains some objects.
    BucketNotEmpty(String),
    /// Storj DCS network rejected the operation because the bucket doens't exist.
    BucketNotFound(String),
    /// Storj DCS network rejected the operation because the object's key contains non-allowed
    /// characters.
    ObjectKeyInvalid(String),
    /// Storj DCS network rejected the operation because it doesn't exists an object in the
    /// specified bucket and key.
    ObjectNotFound(String),
    /// Storj DCS network rejected the operation because there isn't valid permissions for
    /// conducting the requested operation.
    PermissionDenied(String),
    /// Storj DCS network rejected the operation because it would exceed the account's segments
    /// limit.
    SegmentsLimitExceeded(String),
    /// Storj DCS network rejected the operation because it would exceed the account's storage
    /// limit.
    StorageLimitExceeded(String),
    /// Storj DCS network rejected the operation because the specified upload was already completed
    /// or aborted.
    UploadDone(String),

    // Edge service error codes
    /// An Edge Auth service network or protocol error.
    EdgeAuthDialFailed(String),
    /// An Edge Auth service internal error.
    EdgeRegisterAccessFailed(String),

    /// Unknowns isn't an actual code in the FFI constants. It's mostly used to map a code when it
    /// doesn't match any and have not to panic. Callers should report this as a BUG that may be
    /// due to not having updated the FFI to the last version.
    Unknown(String),
}

impl Uplink {
    /// Creates a new instance from the FFI representation.
    ///
    /// The returned instance has a copy of everything that requires from `err`, so it doesn't take
    /// ownership of it. See [implementation design documentation](crate::docs::design).
    pub(crate) fn with_ffi_error(err: &ulksys::UplinkError) -> Self {
        let msg = if err.message.is_null() {
            String::new()
        } else {
            // SAFETY: We trust the FFI that the error contains valid C strings.
            unsafe {
                CStr::from_ptr(err.message)
                    .to_str()
                    .expect("invalid FFI error message; it contains non UTF-8 characters")
                    .to_string()
            }
        };

        match err.code as u32 {
            ulksys::UPLINK_ERROR_INTERNAL => Self::Internal(msg),
            ulksys::UPLINK_ERROR_CANCELED => Self::Canceled(msg),
            ulksys::UPLINK_ERROR_INVALID_HANDLE => Self::InvalidHandle(msg),
            ulksys::UPLINK_ERROR_TOO_MANY_REQUESTS => Self::TooManyRequests(msg),
            ulksys::UPLINK_ERROR_BANDWIDTH_LIMIT_EXCEEDED => Self::BandwidthLimitExceeded(msg),
            ulksys::UPLINK_ERROR_BUCKET_NAME_INVALID => Self::BucketNameInvalid(msg),
            ulksys::UPLINK_ERROR_BUCKET_ALREADY_EXISTS => Self::BucketAlreadyExists(msg),
            ulksys::UPLINK_ERROR_BUCKET_NOT_EMPTY => Self::BucketNotEmpty(msg),
            ulksys::UPLINK_ERROR_BUCKET_NOT_FOUND => Self::BucketNotFound(msg),
            ulksys::UPLINK_ERROR_OBJECT_KEY_INVALID => Self::ObjectKeyInvalid(msg),
            ulksys::UPLINK_ERROR_OBJECT_NOT_FOUND => Self::ObjectNotFound(msg),
            ulksys::UPLINK_ERROR_PERMISSION_DENIED => Self::PermissionDenied(msg),
            ulksys::UPLINK_ERROR_SEGMENTS_LIMIT_EXCEEDED => Self::SegmentsLimitExceeded(msg),
            ulksys::UPLINK_ERROR_STORAGE_LIMIT_EXCEEDED => Self::StorageLimitExceeded(msg),
            ulksys::UPLINK_ERROR_UPLOAD_DONE => Self::UploadDone(msg),
            ulksys::EDGE_ERROR_AUTH_DIAL_FAILED => Self::EdgeAuthDialFailed(msg),
            ulksys::EDGE_ERROR_REGISTER_ACCESS_FAILED => Self::EdgeRegisterAccessFailed(msg),
            _ => Self::Unknown(msg),
        }
    }
}

impl fmt::Display for Uplink {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let (code, details) = match self {
            Self::Internal(msg) => ("internal", msg),
            Self::Canceled(msg) => ("canceled", msg),
            Self::InvalidHandle(msg) => ("invalid handle", msg),
            Self::TooManyRequests(msg) => ("too many requests", msg),
            Self::BandwidthLimitExceeded(msg) => ("bandwidth limit exceeded", msg),
            Self::BucketNameInvalid(msg) => ("bucket name invalid", msg),
            Self::BucketAlreadyExists(msg) => ("bucket already exists", msg),
            Self::BucketNotEmpty(msg) => ("bucket not empty", msg),
            Self::BucketNotFound(msg) => ("bucket not found", msg),
            Self::ObjectKeyInvalid(msg) => ("object key invalid", msg),
            Self::ObjectNotFound(msg) => ("object not found", msg),
            Self::PermissionDenied(msg) => ("permission denied", msg),
            Self::SegmentsLimitExceeded(msg) => ("segments limit exceeded", msg),
            Self::StorageLimitExceeded(msg) => ("storage limit exceeded", msg),
            Self::UploadDone(msg) => ("upload done", msg),
            Self::EdgeAuthDialFailed(msg) => ("dial to auth service failed", msg),
            Self::EdgeRegisterAccessFailed(msg) => ("register access for edge service failed", msg),
            Self::Unknown(msg) => ("unknown", msg),
        };

        write!(f, r#"code: "{}", details: "{}""#, code, details)
    }
}

/// Represents an error that happen because of the violation of an internal assumption.
///
/// An assumption can be violated by the use of a function that returns an error when it should
/// never return it or because it's validated explicitly by the implementation.
///
/// An assumption examples is: a bucket's name returned by the Storj Satellite must always contain
/// UTF-8 valid characters.
#[derive(Debug)]
pub struct Internal {
    /// A human friendly message to provide context of the error.
    pub ctx_msg: String,
    /// The inner error that caused this internal error
    inner: BoxError,
}

impl fmt::Display for Internal {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.ctx_msg)
    }
}

impl stderr::Error for Internal {
    fn source(&self) -> Option<&(dyn stderr::Error + 'static)> {
        Some(self.inner.as_ref())
    }
}
