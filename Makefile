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
clean: integration-tests-env-down
	rm -rf .tmp
	$(MAKE) -C uplink-sys clean
	$(MAKE) -C uplink clean


.PHONY: integration-tests-env-up
integration-tests-env-up:
	docker compose up -d
	$(MAKE) .tmp/env

.PHONY: integration-tests-env-down
integration-tests-env-down:
	docker compose down
	rm -rf .tmp

.tmp/env: .tmp/up/storj-up
	@.tmp/up/storj-up credentials -e | grep -Ei "^export .+" > .tmp/env
	# TODO: This is a hack to get the AWS_* and STORJ_GATEWAY variables without
	# overriding the access grants because those variable are only available using
	# the storj-up from inside of the container, however, we cannot use the access
	# grants because they use the docker compose service name in the URL rather
	# than localhost and then it doesn't resolve.
	# See: https://github.com/storj/up/issues/45#issuecomment-1288808260
	@docker compose exec -T satellite-api storj-up credentials --s3 -e \
		-a http://authservice:8888 -s satellite-api:7777 \
		| grep -E 'AWS|STORJ_GATEWAY' >> .tmp/env

.tmp/up/storj-up: .tmp/up
	cd .tmp/up; go build -tags noquic -o storj-up

.tmp/up:
	mkdir -p .tmp
	cd .tmp; git clone https://github.com/storj/up.git
	cd .tmp/up; git checkout v1.1.0
