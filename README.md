# uplink-rust

[![Actions Status](https://github.com/storj-thirdparty/uplink-rust/workflows/uplink-sys/badge.svg)](https://github.com/storj-thirdparty/uplink-rust/actions)

Originally developed and tested using:
`Ubuntu 20.04`
`Rust 1.51.0`
`Go 1.16.2`
`uplink-c v1.2.3`

Should work with other versions and on OS X but has not been tested yet.

See [uplink-sys README](https://github.com/storj-thirdparty/uplink-rust/tree/main/uplink-sys) for build instructions.

# uplink-sys
The [uplink-sys](https://github.com/storj-thirdparty/uplink-rust/tree/main/uplink-sys) crate provides Rust bindings to [uplink-c](https://github.com/storj/uplink-c).

This crate provides direct unsafe bindings to the C functions provided by uplink-c.

## Usage
See [uplink-sys/examples](https://github.com/storj-thirdparty/uplink-rust/tree/main/uplink-sys/examples) for example projects using the uplink-sys crate.  Below is an example showing how to list buckets using the crate's unsafe C bindings.
```rust
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

        // Free memory
        uplink_sys::uplink_free_bucket(p_bucket_result);
    }

    // Free memory
    uplink_sys::uplink_free_access_result(access_result);
    uplink_sys::uplink_free_project_result(project_result);
    uplink_sys::uplink_free_bucket_iterator(p_bucket_iterator);
    uplink_sys::uplink_free_error(p_bucket_iterator_err);
}
```

# uplink
In the future a safe crate may be added to this repo to wrap the unsafe pointer/memory handling of the sys crate.

# Testing
The project has been tested on the following operating systems:
```
* ubuntu
	* Version: 20.04.2 LTS
	* Processor: Intel® Core™ i7-10510U CPU @ 1.80GHz × 8
* macOS
	* Version: 10.15.7
	* Processor: 2.6 GHz 6-Core Intel Corei7
```
