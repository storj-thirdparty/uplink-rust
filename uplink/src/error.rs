//! Errors returned by this crate.

use std::error as stderr;
use std::fmt;

use uplink_sys as ulksys;

/// The error type that this create use to wrap errors.
#[derive(Debug)]
pub enum Error {
    /// Identifies invalid arguments passed to a function or method.
    InvalidArguments(Args),
    /// Identifies a native error returned by the underlying Uplink C bindings
    /// library.
    Uplink(UplinkErrorDetails),
}

impl Error {
    /// Convenient constructor for creating an InvalidArguments Error.
    /// See [`Args`] documentation to know about the convention for the value of
    /// the `names` parameter because this constructor panics if they are
    /// violated.
    pub(crate) fn new_invalid_arguments(names: &str, msg: &str) -> Self {
        Self::InvalidArguments(Args::new(names, msg))
    }

    /// Convenient constructor for creating an Uplink Error.
    /// It returns None if ulkerr is null.
    pub(crate) fn new_uplink(ulkerr: *mut ulksys::UplinkError) -> Option<Self> {
        UplinkErrorDetails::from_raw(ulkerr).map(Self::Uplink)
    }
}

impl stderr::Error for Error {
    fn source(&self) -> Option<&(dyn stderr::Error + 'static)> {
        match self {
            Error::InvalidArguments { .. } => None,
            Error::Uplink { .. } => None,
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
        }
    }
}

/// Represents invalid arguments regarding the business domain.
///
/// # Example
///
/// ```ignore
/// // This example is ignored because it shows how to return an
/// // InvalidArguments error through the constructor methods that aren't
/// // exported outside of this crate.
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
    /// One or several parameters names; it has several conventions for
    /// expressing the involved parameters.
    ///
    /// * When a specific parameter is invalid its value is the exact parameter
    ///   name.
    /// * When the parameter is a list (vector, array, etc.), the invalid items
    ///   can be __optionally__ indicated using square brackets (e.g. `l[3,5,7]`).
    /// * when the parameter is struct, the invalid fields or method return
    ///   return values can be __optionally__ indicated using curly brackets
    ///   (e.g invalid field: `person{name}`, invalid method return value:
    ///   `person{full_name()}`, invalid fields/methods:
    ///   `employee{name, position()}`).
    /// * When several parameters are invalid, its values is the parameters
    ///   names wrapped in round brackets (e.g. `(p1,p3)`); it also accepts any
    ///   above combination of parameters types
    ///   (e.g. `(p1, l[2,10], person{name})`).
    /// * When all the function parameters are invalid, `<all>` is used.
    ///
    /// For enforcing the conventions across your code base use the
    /// [`Error::new_invalid_arguments`] constructor function.
    pub names: String,
    /// A human friendly message that explains why the argument(s) are invalid.
    pub msg: String,
}

impl Args {
    // TODO: this constructor must enforce the names convention commented in the
    // documentation of this type and panic if they are violated because that
    // means that there is a bug in the code that uses it.
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
            "{} argurments have invalid values. {}",
            self.names, self.msg
        )
    }
}

/// Wraps a native error returned by the underlying Uplink C bindings library
/// providing the access to its details.
#[derive(Debug)]
pub struct UplinkErrorDetails {
    /// The error code returned by the underlying Uplink C bindings library.
    pub code: i32,
    /// The error message returned by the underlying Uplink C bindings library
    /// converted to a String.
    pub details: String,
}

impl UplinkErrorDetails {
    fn from_raw(ulkerr: *mut ulksys::UplinkError) -> Option<Self> {
        if ulkerr.is_null() {
            return None;
        }

        // This is safe because the we have checked just above that the pointer
        // isn't null
        unsafe {
            Some(Self {
                code: (*ulkerr).code,
                details: (*ulkerr).message.as_ref().unwrap().to_string(),
            })
        }
    }

    fn message(&self) -> &str {
        match self.code as u32 {
            ulksys::UPLINK_ERROR_INTERNAL => "internal",
            ulksys::UPLINK_ERROR_CANCELED => "canceled",
            ulksys::UPLINK_ERROR_INVALID_HANDLE => "invalid handle",
            ulksys::UPLINK_ERROR_TOO_MANY_REQUESTS => "too many requests",
            ulksys::UPLINK_ERROR_BANDWIDTH_LIMIT_EXCEEDED => "bandwidth limit exceeded",
            ulksys::UPLINK_ERROR_BUCKET_NAME_INVALID => "invalid bucket name",
            ulksys::UPLINK_ERROR_BUCKET_ALREADY_EXISTS => "bucket already exists",
            ulksys::UPLINK_ERROR_BUCKET_NOT_EMPTY => "bucket not empty",
            ulksys::UPLINK_ERROR_BUCKET_NOT_FOUND => "bucket not found",
            ulksys::UPLINK_ERROR_OBJECT_KEY_INVALID => "invalid object key",
            ulksys::UPLINK_ERROR_OBJECT_NOT_FOUND => "object not found",
            ulksys::UPLINK_ERROR_UPLOAD_DONE => "upload done",
            _ => "unknown",
        }
    }
}

impl fmt::Display for UplinkErrorDetails {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            r#"Uplink error: code: {}, message: "{}", details: "{}""#,
            self.code,
            self.message(),
            self.details,
        )
    }
}
