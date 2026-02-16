.PHONY: clean build test e2etest

clean:
	cargo clean

build:
	cargo build

test:
	cargo test

e2etest:
	$(MAKE) -f tests/e2e.mk
