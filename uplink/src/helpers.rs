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
pub(crate) mod test {
    // Helper functions used test of this crate.
    use std::ffi::CString;
    use std::os::raw::c_char;

    /// Asserts that a C string has the same value than the passed `&str`.
    /// NOTE it doesn't compare memory addresses.
    pub(crate) fn assert_c_string(have: *const c_char, want: &str) {
        let want_c = CString::new(want).expect("want not having any null character");
        let want_raw = want_c.as_ptr();

        assert_raw_pointer(have, want_raw, want.len());
    }

    /// Asserts the two raw pointers point to the same values.
    /// Because it isn't possible to know the length of `have` nor `want`, the
    /// function compare the memory positions until `want_length`.
    /// NOTE it doesn't compare memory addresses.
    fn assert_raw_pointer<T: std::cmp::Eq + Copy>(
        have: *const T,
        want: *const T,
        want_length: usize,
    ) {
        // SAFETY: We are not making any conversion on what the address pointed
        // on each iteration, where we just increment the offset by one and
        // compare the values pointed by `have` and `want` pointers.
        // What it could be wrong is accessing to an offset which point to a
        // forbidden memory address (e.g. not allowed by the OS, etc.), which
        // while we could guarantee the safety leaning on the trust of the
        // caller, which should  pass the correct length for want, the caller
        // cannot gives the guarantee for the `have` pointer because it's what
        // it wants to test.
        unsafe {
            for i in 0..want_length {
                let h = *have.add(i);
                let w = *want.add(i);
                if h != w {
                    panic!("unexpected value in raw pointer memory position +{}", i);
                }
            }
        }
    }

    // Unit tests for helper functions.
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

    #[test]
    fn test_assert_c_string() {
        {
            // Case: Empty string.
            let empty = CString::new("").unwrap();
            assert_c_string(empty.as_ptr(), "");
        }
        {
            // Case: A string of length greater than 0.
            let word = CString::new("Rust").unwrap();
            assert_c_string(word.as_ptr(), "Rust");
        }
    }
}
