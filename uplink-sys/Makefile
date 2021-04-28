UPLINK_C = uplink-c

.PHONY: default
default: build

.PHONY: build
build: $(UPLINK_C)/.git
	cargo build

.PHONY: test
test: $(UPLINK_C)/.git
	cargo test

# Checkout uplink-c submodule if it hasn't been checked out yet
$(UPLINK_C)/.git:
	git submodule update --init

.PHONY: clean
clean:
	cargo clean
	rm -rf $(UPLINK_C)/.build