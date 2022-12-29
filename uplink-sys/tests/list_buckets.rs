use std::env;
use std::ffi::{CStr, CString};
use std::fs;

#[test]
fn list_buckets() {
    let access_grant = env::var("STORJ_ACCESS").expect("STORJ_ACCESS env var isn't defined");
    let access_grant = CString::new(access_grant).expect("CString::new failed");

    unsafe {
        // Parse access grant
        let access_result = uplink_sys::uplink_parse_access(
            access_grant.as_ptr() as *mut uplink_sys::uplink_const_char
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
