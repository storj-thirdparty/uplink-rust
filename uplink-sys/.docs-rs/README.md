# Docs.rs

This directory contains a precompiled uplink-c libraries to build the docs of this crate by
[docs.rs](https://docs.rs).

Using this precompiled libraries avoid to require to install Go in the Docker image used for
building the documentation for this crate.

The libraries binaries are only available for Linux X86_64 architecture because docs.rs is
configured to only build documentation for this target which is enough for the purpose.

See the specific metadata configuration for docs.rs in the `Cargo.toml` file and how building this
crate for docs.rs differs from a usual build in the `build.rs` file.
