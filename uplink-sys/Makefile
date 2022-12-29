# Custom variables #

UPLINK_C = uplink-c
LOCAL_ABS_CRATE_PATH := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# Makefile special variables #

.DEFAULT_GOAL := build
SHELL := /bin/bash

# Targets #

.PHONY: build
build: $(UPLINK_C)/.git
	cargo build

.PHONY: lint
lint:
	cargo fmt --check
	cargo clippy -- -D clippy::all

.PHONY: test
test: $(UPLINK_C)/.git
	$(MAKE) -C .. integration-tests-env-up
	source ../.tmp/env; cargo test

.PHONY: publish-test
publish-test: clean $(UPLINK_C)/.git
	cargo publish --dry-run -vv

.PHONY: _publish-crate
_publish-crate: $(UPLINK_C)/.git
	cargo publish

# Checkout uplink-c submodule if it hasn't been checked out yet
$(UPLINK_C)/.git:
	git submodule update --init

.PHONY: update-libs-docs-rs
update-libs-docs-rs:
	# Use the root of the repository, otherwise uplink-c doesn't have access to
	# its GIT info and Go 1.18 or newer uses it for adding the version information
	# to the binaries.
	docker run --rm -v $(LOCAL_ABS_CRATE_PATH)/../:/uplink-rust -w /uplink-rust/uplink-sys/$(UPLINK_C) golang:1.18-stretch make build
	cp -r $(UPLINK_C)/.build/* .docs-rs/
	docker run --rm -v $(LOCAL_ABS_CRATE_PATH)/$(UPLINK_C):/$(UPLINK_C) -w /$(UPLINK_C) golang:1.18-stretch rm -rf .build

.PHONY: clean
clean:
	@rm -rf uplink-c/.build
	cargo clean
