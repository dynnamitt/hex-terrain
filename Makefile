.PHONY: clean build test coverage coverage-xml inject-updates wasm serve

WASM_OUT = target/wasm

LATEST_TAG := $(shell git tag --sort=-v:refname | grep -m1 '^v[0-9]' || echo "")
VERSION ?= $(if $(LATEST_TAG),$(shell echo $(LATEST_TAG) | awk -F. '{print $$1"."$$2"."$$3+1}'),v0.0.0)

clean:
	cargo clean

build:
	cargo build

test:
	cargo test

coverage:
	cargo tarpaulin --out html --skip-clean
	@echo "Coverage report: tarpaulin-report.html"

coverage-xml:
	cargo tarpaulin --out xml --skip-clean

inject-updates:
	@test -n "$(TAG)" || { echo "Usage: make inject-updates TAG=v0.0.1"; exit 1; }
	sed -n '/^## $(TAG)$$/,/^## /{/^## /d;p}' UPDATES.md \
		| sed -n 's/^- \(.*\)/          <li>\1<\/li>/p' > /tmp/updates.html
	@test -s /tmp/updates.html || { echo "UPDATES.md missing notes for '$(TAG)'"; exit 1; }
	sed -i -e '/__UPDATES__/r /tmp/updates.html' -e '/__UPDATES__/d' web/index.html

wasm:
	cargo build --release --target wasm32-unknown-unknown \
		--no-default-features --features web
	wasm-bindgen --out-dir $(WASM_OUT) --target web \
		target/wasm32-unknown-unknown/release/hex-terrain.wasm
	cp web/index.html $(WASM_OUT)/
	sed -i 's/__VERSION__/$(VERSION)/' $(WASM_OUT)/index.html

serve: wasm
	python3 -m http.server 8080 --directory $(WASM_OUT)
