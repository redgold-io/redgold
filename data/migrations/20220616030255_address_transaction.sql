CREATE TABLE IF NOT EXISTS address_transaction (
                                    address BLOB NOT NULL,
                                    tx_hash BLOB NOT NULL,
                                    time INTEGER NOT NULL,
                                    incoming INTEGER NOT NULL,
                                    PRIMARY KEY (address, tx_hash)
);


CREATE INDEX IF NOT EXISTS address_transaction_address
    ON address_transaction (address DESC);

CREATE INDEX IF NOT EXISTS address_transaction_tx_hash
    ON address_transaction (tx_hash DESC);

CREATE INDEX IF NOT EXISTS address_transaction_time
    ON address_transaction (time DESC);
