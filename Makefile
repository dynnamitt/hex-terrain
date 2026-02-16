.PHONY: clean build test e2etest

clean:
	cargo clean

build:
	cargo build

test:
	cargo test

e2etest:
	bash tests/e2e_entity_count.sh
