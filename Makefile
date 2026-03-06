.PHONY: clean build test wasm serve

WASM_OUT = target/wasm

clean:
	cargo clean

build:
	cargo build

test:
	cargo test

wasm:
	cargo build --release --target wasm32-unknown-unknown \
		--no-default-features --features web
	wasm-bindgen --out-dir $(WASM_OUT) --target web \
		target/wasm32-unknown-unknown/release/hex-terrain.wasm
	cp web/index.html $(WASM_OUT)/

serve: wasm
	python3 -m http.server 8080 --directory $(WASM_OUT)
