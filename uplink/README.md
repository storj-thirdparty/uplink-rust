# Storj Uplink Library for Rust

[![CI Status](https://img.shields.io/github/actions/workflow/status/storj-thirdparty/uplink-rust/uplink.yml?branch=main&style=for-the-badge)](https://github.com/storj-thirdparty/uplink-rust/actions/workflows/uplink.yml)
[![crates.io](https://img.shields.io/crates/v/uplink.svg?style=for-the-badge)](https://crates.io/crates/uplink)
[![docs.rs](https://img.shields.io/docsrs/uplink?style=for-the-badge)](https://docs.rs/uplink)
![Crates.io](https://img.shields.io/crates/d/uplink?style=for-the-badge)

Safe and idiomatic Rust crate library for the [Storj Uplink Library][storj-uplink].

## Current status

This crate has implemented all the functionalities offered by the `uplink-sys`
create and it's fully documented.

It has several unit-tests and integration tests which prove that a big part of
the public API works as expected.

The crate is fully documented and the `docs` contains documentation outside of
the API (types, function, etc.).

We consider its current status beta and it can be use for production systems
with care because, despite of the integration tests, we don't know any reference
that this crate is used in any production application.

If you're using this crate in any of your applications, we'd love that you open
an issue and you tell us about.

## Implementation

This crate wraps the `uplink-sys` crate present in this same repository for
offering an safe and idiomatic Rust [Storj Uplink][storj-uplink].

Because it relies on `uplink-sys` and `uplink-sys` requires [Go](https://golang.org),
using this crate also requires Go.

### Development

For development the only requirements are Rust, Go, and clang.

For running the integration tests you need a Docker version that has the `compose`
command, which is the `docker-compose` tool that it's now integrated in Docker.
The command is needed because the Makefile invoked, but you should be able to
use a Docker version without the `compose` command,  using the `docker-compose`,
however, you will have to run by hand or make an straightforward change in the
Makefile.

When some integration test fails it may provoke the failure of other integration tests to fail in
subsequent runs. This is because the previous executed test which failed left garbage data in the
satellite/edge services. To execute again the tests without having to execute a full clean up, you
may go to the temporary directory _../.tmp/up_ and run `docker compose down` and
`docker compose up -d` and then execute `make test-integrationa`.


[storj-uplink]: https://github.com/storj/uplink
