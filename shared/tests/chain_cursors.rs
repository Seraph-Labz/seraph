//! Integration tests for the chain_cursors high-water mark.
//!
//! These need a real Postgres, so they are `#[ignore]`d and skipped by the
//! default `cargo test` run that CI performs. To run them:
//!
//!     docker compose up -d postgres
//!     DATABASE_URL=postgres://seraph:seraph@localhost:5432/seraph \
//!         cargo test -p seraph-shared --test chain_cursors -- --ignored
use seraph_shared::db;
use sqlx::PgPool;

/// Connect and bring the schema up to date. Migrations are idempotent, so this
/// is safe to call from every test.
async fn setup() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for these tests");

    let pool = db::connect(&url).await.expect("failed to connect");
    db::run_migrations(&pool)
        .await
        .expect("failed to run migrations");

    pool
}

/// Clear a test chain's cursor so each test starts from a known state.
/// Without this the GREATEST in the upsert would carry values across runs.
async fn reset(pool: &PgPool, chain_id: &str) {
    sqlx::query("DELETE FROM chain_cursors WHERE chain_id = $1")
        .bind(chain_id)
        .execute(pool)
        .await
        .expect("failed to clear test cursor");
}

#[tokio::test]
#[ignore = "requires a live Postgres"]
async fn migrations_apply_cleanly() {
    let pool = setup().await;

    // 0004 must have created the table and be recorded as applied.
    let (applied,): (bool,) =
        sqlx::query_as("SELECT success FROM _sqlx_migrations WHERE version = 4")
            .fetch_one(&pool)
            .await
            .expect("migration 4 was not recorded");

    assert!(applied, "migration 4 did not apply successfully");
}

#[tokio::test]
#[ignore = "requires a live Postgres"]
async fn unknown_chain_has_no_cursor() {
    let pool = setup().await;
    let chain = "test-unknown-chain";
    reset(&pool, chain).await;

    let cursor = db::get_chain_cursor(&pool, chain).await.unwrap();
    assert_eq!(cursor, None, "a chain never indexed must have no cursor");
}

#[tokio::test]
#[ignore = "requires a live Postgres"]
async fn cursor_round_trips() {
    let pool = setup().await;
    let chain = "test-roundtrip";
    reset(&pool, chain).await;

    db::upsert_chain_cursor(&pool, chain, 21_000_000)
        .await
        .unwrap();

    let cursor = db::get_chain_cursor(&pool, chain).await.unwrap();
    assert_eq!(cursor, Some(21_000_000));
}

#[tokio::test]
#[ignore = "requires a live Postgres"]
async fn cursor_advances_but_never_rewinds() {
    let pool = setup().await;
    let chain = "test-monotonic";
    reset(&pool, chain).await;

    db::upsert_chain_cursor(&pool, chain, 100).await.unwrap();
    assert_eq!(db::get_chain_cursor(&pool, chain).await.unwrap(), Some(100));

    // Forward progress is recorded.
    db::upsert_chain_cursor(&pool, chain, 200).await.unwrap();
    assert_eq!(db::get_chain_cursor(&pool, chain).await.unwrap(), Some(200));

    // A runner restarting mid-sweep must not rewind progress another runner
    // already made — this is what the GREATEST in the upsert is protecting.
    db::upsert_chain_cursor(&pool, chain, 150).await.unwrap();
    assert_eq!(
        db::get_chain_cursor(&pool, chain).await.unwrap(),
        Some(200),
        "cursor rewound to a lower block"
    );
}

#[tokio::test]
#[ignore = "requires a live Postgres"]
async fn cursor_survives_a_block_number_beyond_u32() {
    let pool = setup().await;
    let chain = "test-large-block";
    reset(&pool, chain).await;

    // Guards the u64 -> BIGINT conversion in upsert_chain_cursor.
    let block = u64::from(u32::MAX) + 1_000;
    db::upsert_chain_cursor(&pool, chain, block).await.unwrap();

    assert_eq!(
        db::get_chain_cursor(&pool, chain).await.unwrap(),
        Some(block)
    );
}
