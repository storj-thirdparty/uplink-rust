# Makefile special variables #

.DEFAULT_GOAL := lint
SHELL := /bin/bash

# Targets #

.PHONY: build
build:
	cargo build

.PHONY: lint
lint:
	cargo fmt --check
	cargo clippy -- -D clippy::all

## Tests ##
.PHONY:test
test: test-doc test-unit test-integration

.PHONY: test-doc
test-doc:
	cargo test --doc

.PHONY: test-integration
test-integration:
	$(MAKE) -C .. integration-tests-env-up
	# We don't use `cargo test --test` because it requires to pass a specific test file name and
	# it doesn't accept a list of files. For being able to only run the integration tests we prefix
	# all the integration tests functions with `integration_` and then filter them with the usual
	# `cargo test` command.
	source ../.tmp/env; cargo test integration

.PHONY: test-unit
test-unit:
	cargo test --lib

## Publish crate ##
.PHONY: publish-test
publish-test:
	cargo publish --dry-run -vv

.PHONY: _publish-crate
_publish-crate:
	cargo publish

## Clean ##
.PHONY: clean
clean:
	cargo clean
