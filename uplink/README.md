# Storj Uplink Library for Rust

[![CI Status](https://img.shields.io/github/workflow/status/storj-thirdparty/uplink-rust/uplink?style=for-the-badge)](https://github.com/storj-thirdparty/uplink-rust/actions/workflows/uplink.yml)
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

- [ ] Access Grant.
  - [ ] Create.
  - [ ] Request an Access Grant with passphrase.
  - [ ] Parse one.
  - [ ] Share one.
  - [ ] Override an encryption key of a specific Bucket and prefix.
- [ ] Project
  - [ ] Create a Bucket.
  - [ ] Try to create a Bucket which already exists.
  - [ ] Ensure a Bucket, an existing and non-existing one.
  - [ ] Stat a Bucket.
  - [ ] List Buckets.
  - [ ] Upload an Object.
  - [ ] Upload an Object with Custom Metadata.
  - [ ] Multipart upload.
  - [ ] Download an Object.
  - [ ] Stat an Object.
  - [ ] List Objects with and without System and Custom Metadata.
  - [ ] Move an object.
  - [ ] Delete an Object.
  - [ ] Delete an empty Bucket.
  - [ ] Delete a Bucket with objects.
- [ ] Edge.
  - [ ] Join a share URL.
  - [ ] Register an Access Grant.

General:

- [X] Add a CI solution (Travis, Github actions, etc.) for running tests,
      linters on every PR and when something is merge into the `main` branch.
- [ ] Add general documentation about the Storj network and its entities
      mimicking the original [Go Uplink package](https://pkg.go.dev/storj.io/uplink#section-documentation).
- [X] Add some documentation about the crate design and implementation if the
      documentation of each module, types, functions, etc., aren't enough.



[storj-uplink]: https://github.com/storj/uplink
