# Makefile special variables #

.DEFAULT_GOAL := build

# Targets #

.PHONY: build
build:
	$(MAKE) -C uplink-sys build
	$(MAKE) -C uplink build

.PHONY: lint
lint:
	$(MAKE) -C uplink-sys lint
	$(MAKE) -C uplink lint

.PHONY: test
test:
	$(MAKE) -C uplink-sys test
	$(MAKE) -C uplink test

.PHONY: clean
clean:
	$(MAKE) -C uplink-sys clean
	$(MAKE) -C uplink clean
