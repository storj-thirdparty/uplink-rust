# uplink-sys

[![CI Status](https://img.shields.io/github/workflow/status/storj-thirdparty/uplink-rust/uplink-sys?style=for-the-badge)](https://github.com/storj-thirdparty/uplink-rust/actions/workflows/uplink-sys.yml)
[![Crates.io](https://img.shields.io/crates/v/uplink-sys?style=for-the-badge)](https://crates.io/crates/uplink-sys)
[![docs.rs](https://img.shields.io/docsrs/uplink-sys?style=for-the-badge)](https://docs.rs/uplink-sys)
![Crates.io](https://img.shields.io/crates/d/uplink-sys?style=for-the-badge)

This crate provides auto-generated unsafe Rust bindings, through [bindgen](https://github.com/rust-lang/rust-bindgen/), to C functions provided by [uplink-c](https://github.com/storj/uplink-c/), the C interface for the Storj uplink API library.

## Building (from repo)

### Linux

 - Install [Go](https://golang.org/doc/install)
 - Install [Rust](https://www.rust-lang.org/tools/install)
 - Install GCC and make
  `sudo apt install build-essential`
 - Install libclang (required by bindgen for generating platform specific c bindings)
  `sudo apt install libclang-dev`
 - Checkout this repo
 - Build crate
  `make build` (from `uplink-sys` directory)

### macOS

 - Install [Go](https://golang.org/doc/install)
 - Install [Rust](https://www.rust-lang.org/tools/install)
 - Checkout this repo
 - Build crate
  `make build` (from `uplink-sys` directory)

## Building (from crates.io)

### Linux

 - Install [Go](https://golang.org/doc/install)
 - Install libclang (required by bindgen for generating platform specific c bindings)
 - Add [uplink-sys](https://crates.io/crates/uplink-sys) to Cargo.toml

## Tests

__NOTE__ the project has been tested on the following operating systems:
```
* ubuntu
	* Version: 20.04.2 LTS
	* Processor: Intel® Core™ i7-10510U CPU @ 1.80GHz × 8
* macOS
	* Version: 10.15.7
	* Processor: 2.6 GHz 6-Core Intel Corei7
```

### Setup

To allow the integrations tests access to the test project, create a file in this directory with the satellite address and api key for running tests.
Do not commit this file to the repo.
`test_secrets.txt`:
```
<satellite_addresss>
<api_key>
```

### Run

`make test`

## Usage

See the [examples directory](https://github.com/storj-thirdparty/uplink-rust/tree/main/uplink-sys/examples) to see how use the `uplink-sys` crate.

Below is an example showing how to list buckets using the crate's unsafe C bindings.

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
