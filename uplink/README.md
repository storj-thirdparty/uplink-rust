# Storj Uplink Library for Rust

Safe and idiomatic Rust crate library for the [Storj Uplink Library][storj-uplink].

__THIS IS WIP__, there is not any estimation of time nor a commitment when this
Rust binding will have all the functionalities, so don't use it for any non-pet
project.

## Implementation

This crate wraps the `uplink-sys` crate present in this same repository for
offering an safe and idiomatic Rust [Storj Uplink][storj-uplink].

### Development plan and status

Entities:

- [ ] [Access](https://pkg.go.dev/storj.io/uplink#Access)
- [ ] [Bucket](https://pkg.go.dev/storj.io/uplink#Bucket)
- [ ] [Bucket Iterator](https://pkg.go.dev/storj.io/uplink#BucketIterator)
- [ ] [Config](https://pkg.go.dev/storj.io/uplink#Config)
- [ ] [Custom Metadata](https://pkg.go.dev/storj.io/uplink#CustomMetadata)
- [ ] [Download](https://pkg.go.dev/storj.io/uplink#Download)
- [ ] [Download Options](https://pkg.go.dev/storj.io/uplink#DownloadOptions)
- [ ] [Encryption Key](https://pkg.go.dev/storj.io/uplink#EncryptionKey)
- [ ] [List Buckets Options](https://pkg.go.dev/storj.io/uplink#ListBucketsOptions)
- [ ] [List Objects Options](https://pkg.go.dev/storj.io/uplink#ListObjectsOptions)
- [ ] [Object](https://pkg.go.dev/storj.io/uplink#Object)
- [ ] [Object Iterator](https://pkg.go.dev/storj.io/uplink#ObjectIterator)
- [ ] [Permission](https://pkg.go.dev/storj.io/uplink#Permission)
- [ ] [Project](https://pkg.go.dev/storj.io/uplink#Project)
- [ ] [Share Prefix](https://pkg.go.dev/storj.io/uplink#SharePrefix)
- [ ] [System Metadata](https://pkg.go.dev/storj.io/uplink#SystemMetadata)
- [ ] [Upload](https://pkg.go.dev/storj.io/uplink#Upload)
- [ ] [Upload Options](https://pkg.go.dev/storj.io/uplink#UploadOptions)

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
  - [ ] Download an Object.
  - [ ] Stat an Object.
  - [ ] List Objects with and without System and Custom Metadata.
  - [ ] Delete an Object.
  - [ ] Delete an empty Bucket.
  - [ ] Delete a Bucket with objects.

General:

- [ ] Add a CI solution (Travis, Github actions, etc.) for running tests,
      linters on every PR and when something is merge into the `main` branch.
- [ ] Add general documentation about the Storj network and its entities
      mimicking the original [Go Uplink package](https://pkg.go.dev/storj.io/uplink#section-documentation).
- [ ] Add some documentation about the crate design and implementation if the
      documentation of each module, types, functions, etc., aren't enough.



[storj-uplink]: https://github.com/storj/uplink
