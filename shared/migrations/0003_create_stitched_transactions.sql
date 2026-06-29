CREATE TABLE IF NOT EXISTS stitched_transactions (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    correlation_id   TEXT        NOT NULL UNIQUE,
    source_chain     TEXT        NOT NULL,
    dest_chain       TEXT        NOT NULL,
    sender_address   TEXT        NOT NULL,
    receiver_address TEXT,
    amount           TEXT,
    token_address    TEXT,
    protocol_id      TEXT        NOT NULL,
    status           TEXT        NOT NULL DEFAULT 'pending'
                                 CHECK (status IN ('pending', 'inflight', 'completed', 'failed')),
    -- Nullable: dest event may not exist yet when the stitcher first creates the row.
    source_event_id  UUID        REFERENCES cross_chain_events (id) ON DELETE SET NULL,
    dest_event_id    UUID        REFERENCES cross_chain_events (id) ON DELETE SET NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_stitched_correlation_id ON stitched_transactions (correlation_id);
CREATE INDEX idx_stitched_sender_address ON stitched_transactions (sender_address);
CREATE INDEX idx_stitched_created_at     ON stitched_transactions (created_at DESC);
CREATE INDEX idx_stitched_live_status    ON stitched_transactions (status)
    WHERE status IN ('pending', 'inflight');
