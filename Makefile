.PHONY: build build-release test test-db clippy fmt fmt-check ci clean

build:
	cargo build --workspace

build-release:
	cargo build --workspace --release

test:
	cargo test --workspace

# Integration tests that need a live Postgres. Not part of `ci` — they are
# #[ignore]d so the default test run stays green without a database.
#   docker compose up -d postgres && make test-db
DATABASE_URL ?= postgres://seraph:seraph@localhost:5432/seraph
test-db:
	DATABASE_URL=$(DATABASE_URL) cargo test --workspace -- --ignored

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

ci: fmt-check clippy test

clean:
	cargo clean
