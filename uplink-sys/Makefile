# Custom variables #

UPLINK_C = uplink-c

# Makefile special variables #

.DEFAULT_GOAL := build

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
	cargo test

.PHONY: publish-test
publish-test: clean $(UPLINK_C)/.git
	cargo publish --dry-run -vv

.PHONY: _publish-crate
_publish-crate: $(UPLINK_C)/.git
	cargo publish

# Checkout uplink-c submodule if it hasn't been checked out yet
$(UPLINK_C)/.git:
	git submodule update --init

.PHONY: clean
clean:
	@rm -rf uplink-c/.build
	cargo clean
