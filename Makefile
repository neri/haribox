.PHONY: help clean clean-web clean-rust wasm build-web full-build test

ROOT := .
RUST_DIR := rust-task
WASM_SRC := $(RUST_DIR)/target/wasm32-unknown-unknown/release/rust_task.wasm
WASM_DST_SRC := src/wasm/rust_task_bg.wasm

help:
	@echo "Available targets:"
	@echo "  make clean       - Remove build artifacts (web + rust + copied wasm)"
	@echo "  make full-build  - Build rust wasm, copy artifacts, then build web app"
	@echo "  make wasm        - Build rust wasm and copy it into src/public"
	@echo "  make build-web   - Run pnpm build"

clean: clean-web clean-rust
	@rm -f $(WASM_DST_SRC)

clean-web:
	@rm -rf dist

clean-rust:
	@rm -rf $(RUST_DIR)/target

wasm:
	@mkdir -p src/wasm
	@cargo build --manifest-path $(RUST_DIR)/Cargo.toml --release --target wasm32-unknown-unknown
	@wasm-bindgen $(WASM_SRC) --out-dir src/wasm --target web

build-web:
	@pnpm build

full-build: wasm build-web

test:
	cargo test --manifest-path $(RUST_DIR)/Cargo.toml

update:
	pnpm update
	cargo update --manifest-path $(RUST_DIR)/Cargo.toml
