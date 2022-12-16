# Storj Uplink Library for Rust

[![CI Status](https://img.shields.io/github/actions/workflow/status/storj-thirdparty/uplink-rust/uplink.yml?branch=main&style=for-the-badge)](https://github.com/storj-thirdparty/uplink-rust/actions/workflows/uplink.yml)
[![crates.io](https://img.shields.io/crates/v/uplink.svg?style=for-the-badge)](https://crates.io/crates/uplink)
[![docs.rs](https://img.shields.io/docsrs/uplink?style=for-the-badge)](https://docs.rs/uplink)
![Crates.io](https://img.shields.io/crates/d/uplink?style=for-the-badge)

Safe and idiomatic Rust crate library for the [Storj Uplink Library][storj-uplink].

## Current status

This crate has implemented all the functionalities offered by the `uplink-sys`
create and it's fully documented.

It has also several unit-tests but it lacks of integration tests (see
[Development plan and status section](#development-plan-and-status)).
Integration test would prove that it works as expected.

We consider its current status beta and we advice that  it's NOT READY for
production yet.

## Implementation

This crate wraps the `uplink-sys` crate present in this same repository for
offering an safe and idiomatic Rust [Storj Uplink][storj-uplink].

Because it relies on `uplink-sys` and `uplink-sys` requires [Go](https://golang.org),
using this crate also requires Go.

### Development requirements

For development the only requirements are Rust and Go.

For running the integration tests you need a Docker version that has the `compose`
command, which is the `docker-compose` tool that it's now integrated in Docker.
The command is needed because the Makefile invoked, but you should be able to
use a Docker version without the `compose` command,  using the `docker-compose`,
however, you will have to run by hand or make an straightforward change in the
Makefile.

### Development plan and status

General entities:

- [X] [Access](https://pkg.go.dev/storj.io/uplink#Access)
- [X] [Bucket](https://pkg.go.dev/storj.io/uplink#Bucket)
- [X] [Bucket Iterator](https://pkg.go.dev/storj.io/uplink#BucketIterator)
- [X] [Commit Upload Options](https://pkg.go.dev/storj.io/uplink#CommitUploadOptions)
- [X] [Config](https://pkg.go.dev/storj.io/uplink#Config)
- [X] [Custom Metadata](https://pkg.go.dev/storj.io/uplink#CustomMetadata)
- [X] [Download](https://pkg.go.dev/storj.io/uplink#Download)
- [X] [Download Options](https://pkg.go.dev/storj.io/uplink#DownloadOptions)
- [X] [Encryption Key](https://pkg.go.dev/storj.io/uplink#EncryptionKey)
- [X] [List Buckets Options](https://pkg.go.dev/storj.io/uplink#ListBucketsOptions)
- [X] [List Objects Options](https://pkg.go.dev/storj.io/uplink#ListObjectsOptions)
- [X] [List Uploads Options](https://pkg.go.dev/storj.io/uplink#ListUploadsOptions)
- [X] [List Upload Parts Options](https://pkg.go.dev/storj.io/uplink#ListUploadPartsOptions)
- [X] [Move Object Options](https://pkg.go.dev/storj.io/uplink#MoveObjectOptions)
- [X] [Object](https://pkg.go.dev/storj.io/uplink#Object)
- [X] [Object Iterator](https://pkg.go.dev/storj.io/uplink#ObjectIterator)
- [X] [Part](https://pkg.go.dev/storj.io/uplink#Part)
- [X] [Part Iterator](https://pkg.go.dev/storj.io/uplink#PartIterator)
- [X] [Part Upload](https://pkg.go.dev/storj.io/uplink#PartUpload)
- [X] [Permission](https://pkg.go.dev/storj.io/uplink#Permission)
- [X] [Project](https://pkg.go.dev/storj.io/uplink#Project)
- [X] [Share Prefix](https://pkg.go.dev/storj.io/uplink#SharePrefix)
- [X] [System Metadata](https://pkg.go.dev/storj.io/uplink#SystemMetadata)
- [X] [Upload](https://pkg.go.dev/storj.io/uplink#Upload)
- [X] [Upload Info](https://pkg.go.dev/storj.io/uplink#UploadInfo)
- [X] [Upload Iterator](https://pkg.go.dev/storj.io/uplink#UploadIterator)
- [X] [Upload Options](https://pkg.go.dev/storj.io/uplink#UploadOptions)


Edge entities:

- [X] [Config](https://pkg.go.dev/storj.io/uplink/edge#Config)
- [X] [Credentials](https://pkg.go.dev/storj.io/uplink/edge#Credentials)
- [X] [Register Access Options](https://pkg.go.dev/storj.io/uplink/edge#RegisterAccessOptions)
- [X] [Share URL Options](https://pkg.go.dev/storj.io/uplink/edge#ShareURLOptions)


Integration tests:

- [X] Access Grant.
  - [X] Request an Access Grant with passphrase.
  - [X] Parse one.
  - [X] Share one.
  - [X] Override an encryption key of a specific Bucket and prefix.
- [X] Project
  - [X] Create a Bucket.
  - [X] Try to create a Bucket which already exists.
  - [X] Ensure a Bucket, an existing and non-existing one.
  - [X] Stat a Bucket.
  - [X] List Buckets.
  - [X] Upload an Object.
  - [X] Upload an Object with Custom Metadata.
  - [X] Multipart upload.
  - [X] Download an Object.
  - [X] Stat an Object.
  - [X] List Objects with System and Custom Metadata.
  - [X] List Objects without System and Custom Metadata.
  - [X] Copy an object.
  - [X] Move an object.
  - [X] Delete an Object.
  - [X] Delete an empty Bucket.
  - [X] Delete a Bucket with objects.
- [X] Edge.
  - [X] Join a share URL.
  - [ ] Register an Access Grant. - NOTE: [waiting to know if we can test it with storj/up](https://github.com/storj/up/issues/59)

General:

- [X] Add a CI solution (Travis, Github actions, etc.) for running tests,
      linters on every PR and when something is merge into the `main` branch.
- [X] Add general documentation about the Storj network and its entities
      mimicking the original [Go Uplink package](https://pkg.go.dev/storj.io/uplink#section-documentation).
- [X] Add some documentation about the crate design and implementation if the
      documentation of each module, types, functions, etc., aren't enough.



[storj-uplink]: https://github.com/storj/uplink
