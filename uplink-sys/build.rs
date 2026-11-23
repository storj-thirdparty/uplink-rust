extern crate bindgen;

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Converts Rust's architecture name to Go's architecture name.
fn rust_arch_to_go_arch(rust_arch: &str) -> &str {
    match rust_arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "riscv64" => "riscv64",
        "loongarch64" => "loong64",
        _ => panic!("Unsupported architecture: {rust_arch}"),
    }
}

fn rust_os_to_go_os<'a>(rust_arch: &str, rust_os: &'a str) -> &'a str {
    if rust_arch.starts_with("wasm") {
        return "js";
    }

    match rust_os {
        "macos" => "darwin",
        _ => rust_os,
    }
}

/// Runs `cmd`, panicking if it cannot be started or if it doesn't exit successfully.
fn run(cmd: &mut Command) {
    let status = cmd
        .status()
        .unwrap_or_else(|err| panic!("Failed to start {cmd:?}: {err}"));
    assert!(status.success(), "{cmd:?} failed: {status}");
}

/// Copies the contents of the directory `src` into `dst`, creating it if it doesn't exist, and makes
/// the copy writable because the sources may be read-only.
fn copy_dir(src: &Path, dst: &Path) {
    run(Command::new("cp").arg("-R").arg(src.join(".")).arg(dst));
    run(Command::new("chmod").args(["-R", "u+w"]).arg(dst));
}

/// Like [`bindgen::CargoCallbacks`], but it ignores the headers under `OUT_DIR`. This script
/// generates them, so they are always newer than the last time that it started, and Cargo would
/// rerun it on every build. `CargoCallbacks::rerun_on_header_files(false)` isn't enough because it
/// still emits the included headers.
#[derive(Debug)]
struct InputCallbacks {
    out_dir: PathBuf,
}

impl bindgen::callbacks::ParseCallbacks for InputCallbacks {
    fn include_file(&self, filename: &str) {
        if !Path::new(filename).starts_with(&self.out_dir) {
            println!("cargo:rerun-if-changed={filename}");
        }
    }

    fn read_env_var(&self, key: &str) {
        println!("cargo:rerun-if-env-changed={key}");
    }
}

fn main() {
    // Set the environment variables for `go` compilation, so that cross-compilation works.
    let host_triple = env::var("HOST").expect("HOST not defined");
    let target_triple = env::var("TARGET").expect("TARGET not defined");
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS not defined");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH not defined");
    let go_arch = rust_arch_to_go_arch(&target_arch);
    let go_os = rust_os_to_go_os(&target_arch, &target_os);

    // If cross-compiling, suggest to the user that they will need to set their sysroot properly.
    if target_triple != host_triple
        && !env::var("BINDGEN_EXTRA_CLANG_ARGS")
            .unwrap_or_default()
            .contains("sysroot")
    {
        panic!("Environment variable `BINDGEN_EXTRA_CLANG_ARGS` should contain `--sysroot=/path/to/target/sysroot` when cross-compiling");
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not defined"));

    // Directory containing uplink-c project for building
    let uplink_c_dir = out_dir.join("uplink-c");

    // Directory containing uplink-c build
    let uplink_c_build = uplink_c_dir.join(".build");

    // Start from a fresh copy of the project in OUT_DIR, because build scripts must not write
    // anywhere else and a previous run may have left a copy there.
    run(Command::new("rm").arg("-rf").arg(&uplink_c_dir));
    copy_dir(Path::new("uplink-c"), &uplink_c_dir);

    // Don't compile the uplink-c libraries when building the docs for not requiring Go to be
    // installed in the Docker image for building them used by docs.rs
    if env::var("DOCS_RS").is_ok() {
        // Use the precompiled uplink-c libraries for building the docs by docs.rs.
        copy_dir(Path::new(".docs-rs"), &uplink_c_build);
    } else {
        // Build uplink-c generates precompiled lib and header files in .build directory.
        // The copy isn't a Git checkout, so Go must not embed version control information: it would
        // use whatever repository encloses OUT_DIR and fail if Git cannot read it. Go supports
        // `-buildvcs=false` since v1.18 and uplink-c requires v1.20.
        let go_flags = env::var("GOFLAGS").unwrap_or_default();
        run(Command::new("make")
            .arg("build")
            .env("GOOS", go_os)
            .env("GOARCH", go_arch)
            .env("GOFLAGS", format!("{go_flags} -buildvcs=false"))
            .current_dir(&uplink_c_dir));
    }

    // Header file with complete API interface
    let uplink_c_header = uplink_c_build.join("uplink/uplink.h");

    // Link (statically) to uplink-c library during build
    println!("cargo:rustc-link-lib=static=uplink");

    // Add uplink-c build directory to library search path
    println!(
        "cargo:rustc-link-search={}",
        uplink_c_build.to_string_lossy()
    );

    // Only rerun when an input changes. Cargo scans directories for changes.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=uplink-c");

    // Manually link to core and security libs on MacOS
    //
    // N.B.: `CARGO_CFG_TARGET_OS` should be read instead of `cfg(target_os = "macos")`. The latter
    // detects the host OS that is building the `build.rs` script, not the target OS.
    if target_os == "macos" {
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
        .header(uplink_c_header.to_string_lossy())
        // Also make the system headers included by main header dependencies of the build
        .parse_callbacks(Box::new(InputCallbacks {
            out_dir: out_dir.clone(),
        }))
        // The FFI values own memory that must be freed exactly once, so they must not be copied implicitly.
        .derive_copy(false)
        // Generate bindings
        .generate()
        .expect("Error generating bindings.")
        // Write bindings to file to be referenced by main build
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Error writing bindings to file.");
}
