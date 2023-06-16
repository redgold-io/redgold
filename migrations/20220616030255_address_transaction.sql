CREATE TABLE IF NOT EXISTS address_transaction (
                                    address BLOB NOT NULL,
                                    tx_hash BLOB NOT NULL,
                                    time INTEGER NOT NULL,
                                    incoming INTEGER NOT NULL,
                                    PRIMARY KEY (address, tx_hash)
);