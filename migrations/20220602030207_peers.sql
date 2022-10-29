CREATE TABLE IF NOT EXISTS peers (
                                     id    BLOB PRIMARY KEY,
                                     secret_trust       DOUBLE,
                                     public_trust       DOUBLE,
                                     trust       DOUBLE,
                                     deterministic_trust       DOUBLE,
                                     reward_address   BLOB,
                                     peer_data       BLOB,
                                     utxo_distance FLOAT,
                                     tx BLOB NOT NULL,
                                     tx_hash BLOB NOT NULL
);