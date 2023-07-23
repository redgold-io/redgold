CREATE TABLE IF NOT EXISTS multiparty_bridge (
                                    txid BLOB PRIMARY KEY,
                                    secondary_txid BLOB,
                                    outgoing INTEGER,
                                    network INTEGER,
                                    source_address BLOB,
                                    destination_address BLOB,
                                    timestamp INTEGER,
                                    amount INTEGER
);
