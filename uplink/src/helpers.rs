//! Helper functions which are used across the modules of this crate.

use crate::Error;

use std::ffi::CString;

/// creates a CString from a function &str function argument and if there is an
/// error it returnns an Error::InvalidArguments with the passed argument's
/// name.
pub fn cstring_from_str_fn_arg(arg_name: &str, arg_val: &str) -> Result<CString, Error> {
    CString::new(arg_val).map_err(|e| {
        Error::new_invalid_arguments(
            arg_name,
            &format!(
                "cannot contains null bytes (0 byte). Null byte found at {}",
                e.nul_position()
            ),
        )
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_cstring_from_str_fn_arg() {
        let val = cstring_from_str_fn_arg("some", "this is fine")
            .expect("returned error on a valid CString");
        assert_eq!(
            val,
            CString::new("this is fine").unwrap(),
            "returned a CString with an invalid value"
        );

        let err = cstring_from_str_fn_arg("some", "this is invalid\0 ")
            .expect_err("returned Ok on an invalid CString");
        if let Error::InvalidArguments(args) = err {
            assert_eq!(
                args.names, "some",
                "invalid Error::InvalidArguments name field value"
            );
            assert_eq!(
                args.msg, "cannot contains null bytes (0 byte). Null byte found at 15",
                "invalid Error::InvalidArguments msg field value"
            )
        } else {
            panic!("expected an Error::InvalidArguments");
        }
    }
}
