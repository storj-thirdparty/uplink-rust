# Makefile special variables #

.DEFAULT_GOAL := lint

# Targets #

.PHONY: build
build:
	cargo build

.PHONY: lint
lint:
	cargo fmt --check
	cargo clippy -- -D clippy::all

.PHONY: test
test:
	cargo test

.PHONY: publish-test
publish-test:
	cargo publish --dry-run -vv

.PHONY: _publish-crate
_publish-crate:
	cargo publish

.PHONY: clean
clean:
	cargo clean
