# seraph
Cross-chain transaction explorer — Rust indexer for EVM chains, Solana, and Cosmos bridge protocols

## Development setup

Enable the repo's git hooks (pre-commit runs `cargo fmt --check` + `cargo clippy -D warnings`, mirroring CI; commit-msg enforces Conventional Commits):

```sh
git config core.hooksPath .githooks
```
