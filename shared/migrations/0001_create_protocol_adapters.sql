CREATE TABLE IF NOT EXISTS protocol_adapters (
    id               TEXT        PRIMARY KEY,
    name             TEXT        NOT NULL,
    chain_runtime    TEXT        NOT NULL CHECK (chain_runtime IN ('evm', 'solana', 'cosmos')),
    supported_chains TEXT[]      NOT NULL DEFAULT '{}',
    enabled          BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed the 8 MVP adapters.
INSERT INTO protocol_adapters (id, name, chain_runtime, supported_chains) VALUES
    ('layerzero-v2', 'LayerZero V2',  'evm',    ARRAY['ethereum','arbitrum','optimism','base','polygon','bsc','avalanche']),
    ('wormhole',     'Wormhole',      'evm',    ARRAY['ethereum','arbitrum','optimism','base','polygon','bsc','solana']),
    ('axelar',       'Axelar',        'evm',    ARRAY['ethereum','arbitrum','optimism','base','polygon','cosmoshub']),
    ('across',       'Across',        'evm',    ARRAY['ethereum','arbitrum','optimism','base','polygon']),
    ('stargate',     'Stargate',      'evm',    ARRAY['ethereum','arbitrum','optimism','base','polygon','bsc','avalanche']),
    ('cctp',         'CCTP',          'evm',    ARRAY['ethereum','arbitrum','optimism','base','solana']),
    ('hop',          'Hop Protocol',  'evm',    ARRAY['ethereum','arbitrum','optimism','base','polygon']),
    ('connext',      'Connext',       'evm',    ARRAY['ethereum','arbitrum','optimism','base','polygon'])
ON CONFLICT (id) DO NOTHING;
