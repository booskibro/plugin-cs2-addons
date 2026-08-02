PLUGIN_NAME = cs2-addons
WASM_OUT    = target/wasm32-wasip1/release/cs2_addons.wasm

.PHONY: all frontend wasm build test lint clean

all: build

frontend:
	cd frontend && npm ci && npm run build

wasm:
	cargo build --target wasm32-wasip1 --release
	cp $(WASM_OUT) $(PLUGIN_NAME).wasm

build: frontend wasm

test:
	cargo test
	cd frontend && npm test

lint:
	cargo clippy --target wasm32-wasip1 -- -D warnings
	cargo clippy -- -D warnings

clean:
	cargo clean
	rm -f $(PLUGIN_NAME).wasm
	rm -rf frontend/dist
