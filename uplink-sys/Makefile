UPLINK_C = uplink-c

.PHONY: default
default: build

.PHONY: build
build:
	cargo build

.PHONY: test
test:
	cargo test

.PHONY: clean
clean:
	cargo clean
	rm -rf $(UPLINK_C)/.build