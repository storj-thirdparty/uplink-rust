use chrono::{DateTime, NaiveDateTime, Utc};
use std::ffi::{CStr, CString};

fn datetime_string_from_unix_time(t: i64) -> String {
    format!(
        "{}",
        DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(t, 0), Utc)
    )
}

// Example project using the uplink-sys crate to use the uplink API to list buckets
fn main() {
    // Access parameters
    let satellite_address = CString::new("SATELLITE ADDRESS HERE").expect("CString::new failed");
    let api_key = CString::new("API KEY HERE").expect("CString::new failed");
    let passphrase = CString::new("PASSPHRASE HERE").expect("CString::new failed");

    unsafe {
        // Request access
        let access_result = uplink_sys::uplink_request_access_with_passphrase(
            satellite_address.as_ptr() as *mut uplink_sys::uplink_const_char,
            api_key.as_ptr() as *mut uplink_sys::uplink_const_char,
            passphrase.as_ptr() as *mut uplink_sys::uplink_const_char,
        );
        if access_result.error != std::ptr::null_mut() {
            println!("Error requesting access: {:?}", *(access_result.error));
        }

        // Access project
        let project_result = uplink_sys::uplink_open_project(access_result.access);
        if project_result.error != std::ptr::null_mut() {
            println!("Error accessing project: {:?}", *(project_result.error));
        }

        // Create empty string for bucket option struct
        let bucket_options_str = CString::new("").expect("CString::new failed");
        let mut bucket_options = uplink_sys::UplinkListBucketsOptions {
            cursor: bucket_options_str.as_ptr(),
        };

        // Request bucket iterator
        let p_bucket_iterator =
            uplink_sys::uplink_list_buckets(project_result.project, &mut bucket_options);

        // Check for valid bucket iterator
        let p_bucket_iterator_err = uplink_sys::uplink_bucket_iterator_err(p_bucket_iterator);
        if p_bucket_iterator_err == std::ptr::null_mut() {
            println!("Valid bucket iterator.");
        } else {
            println!(
                "Invalid bucket iterator. Error: {:?}.",
                *p_bucket_iterator_err
            );
        }

        // Iterate through all buckets
        let mut bucket_count = 0;
        while uplink_sys::uplink_bucket_iterator_next(p_bucket_iterator) {
            bucket_count += 1;

            let p_bucket_result = uplink_sys::uplink_bucket_iterator_item(p_bucket_iterator);
            let bucket_name = CStr::from_ptr((*p_bucket_result).name)
                .to_str()
                .expect("Invalid bucket name C string.");
            let created = datetime_string_from_unix_time((*p_bucket_result).created);

            println!(
                "Bucket {} => name: {}, created: {}",
                bucket_count, bucket_name, created
            );
        }
    }
}
