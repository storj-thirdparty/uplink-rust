use std::ffi::{CStr, CString};
use std::fs;

#[test]
fn list_buckets() {
    // Get secrets for accessing project
    let secrets = fs::read_to_string("test_secrets.txt").unwrap();
    let secrets = secrets.lines().collect::<Vec<&str>>();
    // Access parameters
    let satellite_address = CString::new(secrets[0]).expect("CString::new failed");
    let api_key = CString::new(secrets[1]).expect("CString::new failed");
    let passphrase = CString::new("").expect("CString::new failed");

    unsafe {
        // Request access
        let access_result = uplink_sys::uplink_request_access_with_passphrase(
            satellite_address.as_ptr() as *mut uplink_sys::uplink_const_char,
            api_key.as_ptr() as *mut uplink_sys::uplink_const_char,
            passphrase.as_ptr() as *mut uplink_sys::uplink_const_char,
        );

        assert_eq!(access_result.error, std::ptr::null_mut()); // verify no error

        // Access project
        let project_result = uplink_sys::uplink_open_project(access_result.access);

        assert_eq!(project_result.error, std::ptr::null_mut()); // verify no error

        // Request bucket iterator
        let bucket_options_str = CString::new("").expect("CString::new failed");
        let mut bucket_options = uplink_sys::UplinkListBucketsOptions {
            cursor: bucket_options_str.as_ptr(),
        };
        let p_bucket_iterator =
            uplink_sys::uplink_list_buckets(project_result.project, &mut bucket_options);

        // Check for valid bucket iterator
        let p_bucket_iterator_err = uplink_sys::uplink_bucket_iterator_err(p_bucket_iterator);

        assert_eq!(p_bucket_iterator_err, std::ptr::null_mut()); // verify non-null pointer

        // Iterate through all buckets
        while uplink_sys::uplink_bucket_iterator_next(p_bucket_iterator) {
            let p_bucket_result = uplink_sys::uplink_bucket_iterator_item(p_bucket_iterator);

            assert_ne!(p_bucket_result, std::ptr::null_mut()); // verify non-null pointer
            let _ = CStr::from_ptr((*p_bucket_result).name).to_str().unwrap(); // verify we got good bucket name data

            // Free memory
            uplink_sys::uplink_free_bucket(p_bucket_result);
        }

        // Free memory
        uplink_sys::uplink_free_access_result(access_result);
        uplink_sys::uplink_free_project_result(project_result);
        uplink_sys::uplink_free_bucket_iterator(p_bucket_iterator);
        uplink_sys::uplink_free_error(p_bucket_iterator_err);
    }
}
