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
test-integration: integration-tests-env-up tests/.tmp/env
	# We don't use `cargo test --test` because it requires to pass a specific test file name and
	# it doesn't accept a list of files. For being able to only run the integration tests we prefix
	# all the integration tests functions with `integration_` and then filter them with the usual
	# `cargo test` command.
	source tests/.tmp/env; cargo test integration

.PHONY: test-unit
test-unit:
	cargo test --lib

tests/.tmp/env: tests/.tmp/up/storj-up
	$(MAKE) integration-tests-credentials | grep -Ei "^export .+" > tests/.tmp/env

.PHONY: integration-tests-credentials
integration-tests-credentials: tests/.tmp/up/storj-up
	@cd tests; .tmp/up/storj-up credentials -e

.PHONY: integration-tests-env-up
integration-tests-env-up:
	cd tests; docker compose up -d

.PHONY: .integration-tests-env-down
.integration-tests-env-down:
	cd tests; docker compose down

tests/.tmp/up/storj-up: tests/.tmp/up
	cd tests/.tmp/up; go build -o storj-up

tests/.tmp/up:
	mkdir -p tests/.tmp
	cd tests/.tmp; git clone https://github.com/storj/up.git
	# Checkout this commit because storj-up was working fine, the latest commit in
	# main at this time is broken.
	cd tests/.tmp/up; git checkout 37026d04259495be27d978a6950fae3205322be1

## Publish crate ##
.PHONY: publish-test
publish-test:
	cargo publish --dry-run -vv

.PHONY: _publish-crate
_publish-crate:
	cargo publish

## Clean ##
.PHONY: clean
clean: .integration-tests-env-down
	rm -rf tests/.tmp
	cargo clean
