CREATE TABLE IF NOT EXISTS transaction_edge
(
                                    transaction_hash BLOB,
                                    output_index INTEGER,
                                    address BLOB,
                                    child_transaction_hash BLOB,
                                    child_input_index INTEGER,
                                    time INTEGER,
                                    PRIMARY KEY (transaction_hash, output_index)
);