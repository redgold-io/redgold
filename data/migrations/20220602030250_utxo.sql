CREATE TABLE IF NOT EXISTS utxo (
                                    transaction_hash BLOB NOT NULL,
                                    output_index INTEGER NOT NULL,
                                    address    BLOB NOT NULL,
                                    output    BLOB NOT NULL,
                                    time INTEGER,
                                    amount INTEGER,
                                    raw BLOB NOT NULL,
                                    PRIMARY KEY (transaction_hash, output_index)
)