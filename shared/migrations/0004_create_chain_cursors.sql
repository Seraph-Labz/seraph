-- High-water mark per chain: the last block the indexer has fully swept.
--
-- The reconcile sweep resumes from here on every reconnect and on process
-- restart, which is what makes a dropped WebSocket lossless. Without it a
-- restart either re-scans from genesis or silently skips the downtime window.
CREATE TABLE IF NOT EXISTS chain_cursors (
    chain_id   TEXT        PRIMARY KEY,
    last_block BIGINT      NOT NULL CHECK (last_block >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
