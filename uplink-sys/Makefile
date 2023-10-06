# Custom variables #

UPLINK_C = uplink-c
LOCAL_ABS_CRATE_PATH := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# Makefile special variables #

.DEFAULT_GOAL := build
SHELL := /usr/bin/env bash

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
	# We cannot build directly with the default user of the container (root)
	# because Git v2.35.2 set  by default that users that aren't owner of the
	# repository can execute Git commands; despite it can be disabled with a
	# global configuration, it seems that Go doesn't obey it and fails getting the
	# version information from the vcs.
	# Because we are not using the root user in the container, our user doesn't
	# have permissions for creating the Go cache directory, so we have to map it
	# to a directory the host machine.
	mkdir -p ../.tmp/go-cache
	docker run --rm -u $$(id -u):$$(id -g) -v $(LOCAL_ABS_CRATE_PATH)/../.tmp/go-cache:/.cache -v $(LOCAL_ABS_CRATE_PATH)/../:/uplink-rust -w /uplink-rust/uplink-sys/uplink-c golang:1.21 make build
	cp -r $(UPLINK_C)/.build/* .docs-rs/
	rm -rf ../.tmp/go-cache uplink-c/.build

.PHONY: clean
clean:
	@rm -rf uplink-c/.build .tmp
	cargo clean
