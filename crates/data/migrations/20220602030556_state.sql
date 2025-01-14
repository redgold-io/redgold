CREATE TABLE IF NOT EXISTS state (
                                    address    BLOB NOT NULL,
                                    selector_hash  BLOB,
                                    state_hash BLOB NOT NULL,
                                    transaction_marker BLOB NOT NULL,
                                    time INTEGER NOT NULL,
                                    index_counter INTEGER NOT NULL,
                                    state BLOB NOT NULL,
                                    PRIMARY KEY (address, selector_hash, index_counter)
);

CREATE INDEX idx_address_nonce_desc ON state(address ASC, index_counter DESC);