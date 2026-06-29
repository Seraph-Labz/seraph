CREATE TABLE IF NOT EXISTS cross_chain_events (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    source_tx_hash   TEXT        NOT NULL,
    source_chain     TEXT        NOT NULL,
    dest_chain       TEXT,
    sender_address   TEXT        NOT NULL,
    receiver_address TEXT,
    -- Stored as TEXT to preserve u256 precision without loss.
    amount           TEXT,
    token_address    TEXT,
    protocol_id      TEXT        NOT NULL,
    correlation_id   TEXT        NOT NULL,
    status           TEXT        NOT NULL DEFAULT 'pending'
                                 CHECK (status IN ('pending', 'inflight', 'completed', 'failed')),
    metadata         JSONB       NOT NULL DEFAULT '{}',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_event_source UNIQUE (source_tx_hash, source_chain)
);

CREATE INDEX idx_events_source_tx_hash ON cross_chain_events (source_tx_hash);
CREATE INDEX idx_events_correlation_id ON cross_chain_events (correlation_id);
CREATE INDEX idx_events_sender_address ON cross_chain_events (sender_address);
CREATE INDEX idx_events_created_at     ON cross_chain_events (created_at DESC);
CREATE INDEX idx_events_protocol_id    ON cross_chain_events (protocol_id);
-- Partial index covers only live events — excludes the majority of completed rows.
CREATE INDEX idx_events_live_status    ON cross_chain_events (status)
    WHERE status IN ('pending', 'inflight');

-- TimescaleDB hypertable (requires the TimescaleDB extension to be enabled in Supabase).
-- Run this manually once the extension is confirmed active:
--   SELECT create_hypertable('cross_chain_events', 'created_at', if_not_exists => TRUE);
