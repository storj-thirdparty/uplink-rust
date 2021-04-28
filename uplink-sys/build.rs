extern crate bindgen;

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Directory containing uplink-c project
    let uplink_c_dir = PathBuf::from("uplink-c");

    // Build uplink-c
    // generates precompiled lib and header files in .build directory
    Command::new("make")
        .arg("build")
        .current_dir(&uplink_c_dir)
        .status()
        .expect("Failed to run make command from build.rs.");

    // Directory containting uplink-c build
    let uplink_c_build = uplink_c_dir.join(".build");

    // Header file with complete API interface
    let uplink_c_header = uplink_c_build.join("uplink/uplink.h");

    // Link (statically) to uplink-c library during build
    println!("cargo:rustc-link-lib=static=uplink");

    // Add uplink-c build directory to library search path
    println!(
        "cargo:rustc-link-search={}",
        uplink_c_build.to_string_lossy()
    );

    // Make uplink-c interface header a dependency of the build
    println!(
        "cargo:rerun-if-changed={}",
        uplink_c_header.to_string_lossy()
    );

    // Manually link to core and security libs on MacOS
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-flags=-l framework=CoreFoundation -l framework=Security");
    }

    bindgen::Builder::default()
        // Use 'allow lists' to avoid generating bindings for system header includes
        // a lot of which isn't required and can't be handled safely anyway.
        // uplink-c uses consistent naming so whitelisting is much easier than blacklisting.
        // All uplink types start with Uplink
        .allowlist_type("Uplink.*")
        // except for uplink_const_char
        .allowlist_type("uplink_const_char")
        // All uplink functions start with uplink_
        .allowlist_function("uplink_.*")
        // This header file is the main API interface and includes all other header files that are required
        // (bindgen runs c preprocessor so we don't need to include nested headers)
        .header("uplink-c/.build/uplink/uplink.h")
        // Also make headers included by main header dependencies of the build
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Generate bindings
        .generate()
        .expect("Error generating bindings.")
        // Write bindings to file to be referenced by main build
        .write_to_file(
            PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not defined")).join("bindings.rs"),
        )
        .expect("Error writing bindings to file.");
}
