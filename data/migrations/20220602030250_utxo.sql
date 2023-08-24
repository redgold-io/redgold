CREATE TABLE IF NOT EXISTS utxo (
                                    transaction_hash BLOB NOT NULL,
                                    output_index INTEGER NOT NULL,
                                    address    BLOB NOT NULL,
                                    output    BLOB NOT NULL,
                                    time INTEGER NOT NULL,
                                    amount INTEGER,
                                    raw BLOB NOT NULL,
                                    has_code INTEGER NOT NULL,
                                    PRIMARY KEY (transaction_hash, output_index)
);

CREATE UNIQUE INDEX IF NOT EXISTS utxo_address
    ON utxo (address DESC);


CREATE UNIQUE INDEX IF NOT EXISTS utxo_transaction_hash
    ON utxo (transaction_hash DESC);

CREATE UNIQUE INDEX IF NOT EXISTS utxo_time
    ON utxo (time DESC);
