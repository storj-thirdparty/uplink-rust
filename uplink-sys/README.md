# uplink-sys

[![Actions Status](https://github.com/storj-thirdparty/uplink-rust/workflows/uplink-sys%20build/badge.svg)](https://github.com/storj-thirdparty/uplink-rust/actions)

This crate provides Rust bindings to [uplink-c](https://github.com/storj/uplink-c/), the C interface for the storj uplink API library.

[TODO]() is the safe wrapper crate for this library.

# Building
## Linux
 - Install [Go](https://golang.org/doc/install)
 - Install [Rust](https://www.rust-lang.org/tools/install)  
 - Install GCC and make  
  `sudo apt install build-essential`
 - Install libclang (required by bindgen for generating platform specific c bindings)  
  `sudo apt install libclang-dev`
 - Build crate  
  `make build` (from `uplink-sys` directory)
  
## Examples
For a usage example see `examples/list_buckets`.  This contains a rust project that lists buckets for a project, you just need to add access parameters.
[TODO]() is a safe library crate wrapping this sys crate so more examples using the wrapper library can be found there.