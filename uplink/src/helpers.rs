//! Helper functions which are used across the modules of this crate.

use crate::Error;

use std::ffi::CString;
use std::os::raw::c_char;

/// creates a CString from a function &str function argument and if there is an
/// error it returns an Error::InvalidArguments with the passed argument's
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

/// Create a heap allocated `str` from a C string of the specified length.
///
/// The function is unsafe because:
/// * It doesn't check for the end NULL byte as it doesn't stop if a NULL byte
///   is before the end of the string.
/// * It doesn't check the characters to be UTF-8 valid, if the string contains
///   invalid UTF-8 bytes then the resulting `str` would have non-deterministic
///   character value on their position.
/// * It will read all the bytes of memory region from pointer to length, so
///   if length is larger than the region, some garbage bytes will be read.
pub unsafe fn unchecked_ptr_c_char_and_length_to_str(
    c_chars: *const c_char,
    length: usize,
) -> Box<str> {
    let mut chars = String::with_capacity(length);

    for i in 0..length as isize {
        chars.push(*c_chars.offset(i) as u8 as char)
    }

    chars.into_boxed_str()
}

#[cfg(test)]
pub(crate) mod test {
    /// Asserts that a C string has the same value than the passed `&str`.
    /// It internally uses `compare_c_string`, panicking when it returns `Some`.
    /// Read its docs for the implications of this function.
    pub(crate) fn assert_c_string(have: *const c_char, want: &str) {
        if let Some((p, h, w)) = compare_c_string(have, want) {
            panic!(
                "unexpected character at position +{}. Want= {:?}, have= {:?}",
                p, w as u8 as char, h as u8 as char,
            );
        }
    }

    /// Asserts the two raw pointers point to the same values.
    /// It internally uses `compare_raw_pointers`, panicking when it returns
    /// `Some`. Read its docs for the implications of this function.
    fn assert_raw_pointer<T: std::cmp::Eq + Copy + std::fmt::Debug>(
        have: *const T,
        want: *const T,
        want_length: usize,
    ) {
        if let Some((p, h, w)) = compare_raw_pointers(have, want, want_length) {
            panic!(
                "unexpected value at memory position +{}. Want= {:?}, have= {:?}",
                p, w, h
            );
        }
    }

    /// Compares that a C string has the same value than the passed `&str`.
    /// It returns `Some` when they don't match, providing a tuple with the
    /// first unmatched position and the value of `c_str` and `r_str` at that
    /// position respectively.
    ///
    /// Because it isn't possible to know the length of `c_str`, it only
    /// compares the memory positions until `r_str`'s length.
    pub(crate) fn compare_c_string(
        c_str: *const c_char,
        r_str: &str,
    ) -> Option<(usize, c_char, c_char)> {
        let c_r_str = CString::new(r_str).expect("want not having any null character");

        compare_raw_pointers(c_str, c_r_str.as_ptr(), r_str.len())
    }

    /// Compares if two raw pointers point to the same values.
    /// It returns `Some` when they don't match, providing a tuple with the
    /// first unmatched position and the value of `a` and `b` at that position
    /// respectively.
    ///
    /// Because it isn't possible to know the length of `a` nor `b`, it only
    /// compares the memory positions until `length`.
    /// NOTE it compares their values, not their memory addresses.
    pub(crate) fn compare_raw_pointers<T: std::cmp::Eq + Copy + std::fmt::Debug>(
        a: *const T,
        b: *const T,
        length: usize,
    ) -> Option<(usize, T, T)> {
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
            for i in 0..length {
                let ai = *a.add(i);
                let bi = *b.add(i);
                if ai != bi {
                    return Some((i, ai, bi));
                }
            }
        }

        None
    }

    // Unit tests for helper functions.
    use super::*;
    use std::ffi::CStr;

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
    fn test_unchecked_ptr_c_char_and_length_to_str() {
        // SAFETY: The function under test is unsafe so everything is wrapped
        // inside of unsafe because there is a minimal logic for each test case.
        unsafe {
            {
                // Case: Exact length.
                let expected = "Storj Uplink Rust";
                let cstr = CStr::from_bytes_with_nul_unchecked(expected.as_bytes());
                let chars = cstr.as_ptr();
                assert_eq!(
                    unchecked_ptr_c_char_and_length_to_str(chars, expected.len()).as_ref(),
                    expected,
                    "str value doesn't match"
                );
            }
            {
                // Case: Exact length and with NULL terminated char.
                let expected = "Storj Uplink Rust\0";
                let cstr = CStr::from_bytes_with_nul_unchecked(expected.as_bytes());
                let chars = cstr.as_ptr();

                assert_eq!(
                    unchecked_ptr_c_char_and_length_to_str(chars, expected.len()).as_ref(),
                    expected,
                    "str value doesn't match"
                );
            }
            {
                // Case: Exact length and with interior NULL chars.
                let expected = "Storj Uplink\0 Ru\0st";
                let cstr = CStr::from_bytes_with_nul_unchecked(expected.as_bytes());
                let chars = cstr.as_ptr();

                assert_eq!(
                    unchecked_ptr_c_char_and_length_to_str(chars, expected.len()).as_ref(),
                    expected,
                    "str value doesn't match"
                );
            }
            {
                // Case: Exact length and with interior and interior and terminated
                // NULL chars.
                let expected = "Storj Uplink\0 Ru\0st\0";
                let cstr = CStr::from_bytes_with_nul_unchecked(expected.as_bytes());
                let chars = cstr.as_ptr();

                assert_eq!(
                    unchecked_ptr_c_char_and_length_to_str(chars, expected.len()).as_ref(),
                    expected,
                    "str value doesn't match"
                );
            }
            {
                // Case: Shorter length.
                let expected = "Storj Uplink Rust";
                let cstr = CStr::from_bytes_with_nul_unchecked(expected.as_bytes());
                let chars = cstr.as_ptr();

                assert_eq!(
                    unchecked_ptr_c_char_and_length_to_str(chars, expected.len() - 1).as_ref(),
                    &expected[..expected.len() - 1],
                    "str value doesn't match"
                );
            }
            {
                // Case: Larger length.
                let expected = "Storj Uplink Rust OUT";
                let cstr = CStr::from_bytes_with_nul_unchecked(expected.as_bytes());
                let chars = cstr.as_ptr();

                // Because the pointer points to `expected` despite that chars only pointed to the
                // firsts 17 characters, the function is receiving a greater length
                // value that it should be so it reads the continuous memory.
                assert_eq!(
                    unchecked_ptr_c_char_and_length_to_str(chars, expected.len()).as_ref(),
                    expected,
                    "str value doesn't match"
                );
            }
            {
                // Case: Invalid UTF-8
                let expected = "Storj Uplink \u{FFFD} Rust";
                let cstr = CStr::from_bytes_with_nul_unchecked(expected.as_bytes());
                let chars = cstr.as_ptr();

                // The values aren't equal because non valid UTF-8 bytes produce a
                // non-deterministic output.
                assert_ne!(
                    unchecked_ptr_c_char_and_length_to_str(chars, expected.len()).as_ref(),
                    expected,
                );
            }
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

    #[test]
    #[should_panic]
    fn test_assert_c_string_panic_shorter() {
        let word = CString::new("Rust").unwrap();
        assert_c_string(word.as_ptr(), "Rusty");
    }

    #[test]
    #[should_panic = "unexpected character at position +1. Want= 'u', have= 'o'"]
    fn test_assert_c_string_panic_unmatch() {
        let word = CString::new("Rost").unwrap();
        assert_c_string(word.as_ptr(), "Rust");
    }

    #[test]
    fn test_assert_raw_pointer() {
        let want = vec![10, 20, 30];
        let have = want.clone();

        assert_raw_pointer(have.as_ptr(), want.as_ptr(), want.len());
        assert_raw_pointer(want.as_ptr(), want.as_ptr(), want.len());
    }

    #[test]
    #[should_panic = "unexpected value at memory position +3. Want= 3, have= 6"]
    fn test_assert_raw_pointer_panic_unmatch() {
        let have = vec![0, 1, 2, 6, 4];
        let want = vec![0, 1, 2, 3, 4];

        assert_raw_pointer(have.as_ptr(), want.as_ptr(), want.len());
    }
}
