.PHONY: build build-release test clippy fmt fmt-check ci clean

build:
	cargo build --workspace

build-release:
	cargo build --workspace --release

test:
	cargo test --workspace

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

ci: fmt-check clippy test

clean:
	cargo clean
