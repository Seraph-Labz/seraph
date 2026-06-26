## What
<!-- What does this PR do? One sentence is enough. -->

## Why
<!-- Why is this change needed? Link a ticket or describe the motivation. -->

## How
<!-- Non-obvious implementation decisions worth explaining. Skip if the diff is self-evident. -->

## Testing
<!-- How was this verified? Unit tests, manual RPC test, local indexer run, etc. -->

## Checklist

**Always**
- [ ] `cargo clippy --workspace --all-targets` passes with no warnings
- [ ] `cargo test --workspace` passes
- [ ] `.env.example` updated if new env vars introduced

**New protocol adapter**
- [ ] `ProtocolAdapter` trait fully implemented — no `todo!()` or `unimplemented!()`
- [ ] Added to enum dispatch in `src/adapters/mod.rs`
- [ ] Registered in the relevant chain runtime (`evm/`, `solana/`, or `cosmos/`)
- [ ] `protocol_adapters` table row added (migration or seed)
- [ ] `correlation_id()` returns a stable, deterministic value for the protocol

**Database / schema changes**
- [ ] Migration file added under `shared/migrations/`
- [ ] `sqlx-data.json` regenerated (`cargo sqlx prepare --workspace`)
- [ ] Backward-compatible or includes a rollback plan