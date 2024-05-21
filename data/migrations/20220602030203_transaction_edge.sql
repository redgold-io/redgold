CREATE TABLE IF NOT EXISTS transaction_edge
(
                                    transaction_hash BLOB NOT NULL,
                                    output_index INTEGER NOT NULL,
                                    address BLOB NOT NULL,
                                    child_transaction_hash BLOB NOT NULL,
                                    child_input_index INTEGER NOT NULL,
                                    time INTEGER NOT NULL,
                                    PRIMARY KEY (transaction_hash, output_index)
);


CREATE INDEX IF NOT EXISTS transaction_edge_transaction_hash
    ON transaction_edge (transaction_hash DESC);

CREATE INDEX IF NOT EXISTS transaction_edge_child_transaction_hash
    ON transaction_edge (child_transaction_hash DESC);

CREATE INDEX IF NOT EXISTS transaction_edge_time
    ON transaction_edge (time DESC);

CREATE INDEX IF NOT EXISTS transaction_edge_address
    ON transaction_edge (address DESC);
