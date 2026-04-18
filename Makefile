.PHONY: clean build test coverage coverage-xml inject-updates wasm wasm-deps serve \
	svg-preview svg-plain svg-rich svg-json svg-html svg-prep

WASM_OUT = target/wasm
WASM_BINDGEN_VER := $(shell grep -A1 '^name = "wasm-bindgen"$$' Cargo.lock | grep version | head -1 | cut -d'"' -f2)

LATEST_TAG := $(shell git tag --sort=-v:refname | grep -m1 '^v[0-9]' || echo "")
VERSION ?= $(if $(LATEST_TAG),$(shell echo $(LATEST_TAG) | awk -F. '{print $$1"."$$2"."$$3+1}'),v0.0.0)

SVG_OUT ?= target/svg-preview
SVG_RADIUS ?= 2
SVG_PAD ?= 0.6
SHORT_SHA ?= $(shell git rev-parse --short HEAD 2>/dev/null || echo local)

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

wasm-deps:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-bindgen-cli --version $(WASM_BINDGEN_VER)

wasm:
	cargo build --release --target wasm32-unknown-unknown \
		--no-default-features --features web \
		|| { echo "Build failed — installing wasm deps and retrying..."; \
		     $(MAKE) wasm-deps && cargo build --release --target wasm32-unknown-unknown \
		     --no-default-features --features web; }
	wasm-bindgen --out-dir $(WASM_OUT) --target web \
		target/wasm32-unknown-unknown/release/hex-terrain.wasm \
		|| { echo "wasm-bindgen not found — installing and retrying..."; \
		     cargo install wasm-bindgen-cli --version $(WASM_BINDGEN_VER) && wasm-bindgen --out-dir $(WASM_OUT) --target web \
		     target/wasm32-unknown-unknown/release/hex-terrain.wasm; }
	cp -r assets $(WASM_OUT)/
	cp web/index.html $(WASM_OUT)/
	sed -i 's/__VERSION__/$(VERSION)/' $(WASM_OUT)/index.html

serve: wasm
	python3 -m http.server 8080 --directory $(WASM_OUT)

svg-prep:
	@mkdir -p $(SVG_OUT)

svg-plain: svg-prep
	cargo run -q -p hex-grid --example svg --release -- $(SVG_RADIUS) $(SVG_PAD) > $(SVG_OUT)/hex-grid.svg

svg-rich: svg-prep
	cargo run -q -p hex-grid --example svg --release -- $(SVG_RADIUS) $(SVG_PAD) --rich > $(SVG_OUT)/hex-grid-rich.svg

svg-json: svg-prep
	cargo run -q -p hex-grid --example svg --release -- $(SVG_RADIUS) $(SVG_PAD) --json > $(SVG_OUT)/hex-grid.json

svg-html: svg-prep
	sed "s|__SHA__|$(SHORT_SHA)|g" web/svg-preview.html > $(SVG_OUT)/index.html

svg-preview: svg-plain svg-rich svg-json svg-html
	@echo "svg preview built in $(SVG_OUT)/"
