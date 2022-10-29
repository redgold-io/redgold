CREATE TABLE IF NOT EXISTS utxo (
                                    transaction_hash BLOB,
                                    output_index INTEGER,
                                    address    BLOB,
                                    output    BLOB,
                                    time INTEGER,
                                    PRIMARY KEY (transaction_hash, output_index)
)