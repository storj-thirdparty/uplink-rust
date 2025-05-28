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
	$(MAKE) -C uplink-sys clean
	$(MAKE) -C uplink clean


.PHONY: integration-tests-env-up
integration-tests-env-up: .tmp/up/docker-compose.yaml
	cd .tmp/up; docker compose up -d --wait --wait-timeout 120 \
		|| { echo "Docker compose services failed to start."; docker compose down; exit 1; }
	$(MAKE) .tmp/env

.PHONY: integration-tests-env-down
integration-tests-env-down:
	@if [ -f .tmp/up/docker-compose.yaml ]; then cd .tmp/up; docker compose down; fi
	rm -rf .tmp

.tmp/env: .tmp/up/storj-up
	@cd .tmp/up; ./storj-up credentials -p -e | grep -Ei "^export .+" > ../env
	# TODO: This is a hack to get the AWS_* and STORJ_GATEWAY variables without
	# overriding the access grants because those variable are only available using
	# the storj-up from inside of the container, however, we cannot use the access
	# grants because they use the docker compose service name in the URL rather
	# than localhost and then it doesn't resolve.
	# See: https://github.com/storj/up/issues/45#issuecomment-1288808260
	@cd .tmp/up; docker compose exec -T satellite-api storj-up credentials --s3 -e \
		-a http://authservice:8888 -s satellite-api:7777 -c satellite-api:10000 \
		| grep -E 'AWS|STORJ_GATEWAY' >> ../env

.tmp/up/storj-up: .tmp/up
	cd .tmp/up; go build -tags noquic -o storj-up
	cd .tmp/up; ./storj-up init minimal,edge,db

.tmp/up:
	mkdir -p .tmp
	cd .tmp; git clone https://github.com/storj/up.git
	cd .tmp/up; git checkout v1.2.7

.tmp/up/docker-compose.yaml: .tmp/up/storj-up
