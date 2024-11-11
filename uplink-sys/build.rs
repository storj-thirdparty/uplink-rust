extern crate bindgen;

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not defined"));

    // Directory containing uplink-c project source
    let uplink_c_src = PathBuf::from("uplink-c");

    // Don't compile the uplink-c libraries when building the docs for not requiring Go to be
    // installed in the Docker image for building them used by docs.rs
    if env::var("DOCS_RS").is_err() {
        // Build uplink-c generates precompiled lib and header files in .build directory.
        // We execute the command in its directory because go build, from v1.18, embeds version control
        // information and the command fails if `-bildvcs=false` isn't set. We don't want to pass the
        // command-line flag because then it would fail when using a previous Go version.
        // Copying and building from a copy it doesn't work because it's a git submodule, hence it uses
        // a relative path to the superproject unless that the destination path is under the same
        // parent tree directory and with the same depth.
        Command::new("make")
            .arg("build")
            .current_dir(&uplink_c_src)
            .status()
            .expect("Failed to run make command from build.rs.");
    }

    // Directory containing uplink-c project for building
    let uplink_c_dir = out_dir.join("uplink-c");
    // Copy project to OUT_DIR for building
    Command::new("cp")
        .args([
            "-Rf",
            &uplink_c_src.to_string_lossy(),
            &uplink_c_dir.to_string_lossy(),
        ])
        .status()
        .expect("Failed to copy uplink-c directory.");

    if env::var("DOCS_RS").is_ok() {
        // Use the precompiled uplink-c libraries for building the docs by docs.rs.
        Command::new("cp")
            .args([
                "-R",
                &PathBuf::from(".docs-rs").to_string_lossy(),
                &uplink_c_dir.join(".build").to_string_lossy(),
            ])
            .status()
            .expect("Failed to copy docs-rs precompiled uplink-c lib binaries");
    } else {
        // Delete the generated build files for avoiding `cargo publish` to complain about modifying
        // things outside of the OUT_DIR.
        Command::new("rm")
            .args(["-r", &uplink_c_src.join(".build").to_string_lossy()])
            .status()
            .expect("Failed to delete  uplink-c/.build directory.");
    }

    // Directory containing uplink-c build
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
        // uplink-c uses consistent naming so an allow list is much easier than a block list.
        // All uplink types start with Uplink
        .allowlist_type("Uplink.*")
        // All edge services types start with Edge
        .allowlist_type("Edge.*")
        // except for uplink_const_char
        .allowlist_type("uplink_const_char")
        // All uplink functions start with uplink_
        .allowlist_function("uplink_.*")
        // All edge services functions start with edge_
        .allowlist_function("edge_.*")
        // Uplink error code #define's start with UPLINK_ERROR_
        .allowlist_var("UPLINK_ERROR_.*")
        // Edge services error code #define's start with EDGE_ERROR_
        .allowlist_var("EDGE_ERROR_.*")
        // This header file is the main API interface and includes all other header files that are required
        // (bindgen runs c preprocessor so we don't need to include nested headers)
        .header(
            uplink_c_dir
                .join(".build/uplink/uplink.h")
                .to_string_lossy(),
        )
        // Also make headers included by main header dependencies of the build
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Generate bindings
        .generate()
        .expect("Error generating bindings.")
        // Write bindings to file to be referenced by main build
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Error writing bindings to file.");
}
