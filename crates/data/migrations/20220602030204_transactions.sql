CREATE TABLE IF NOT EXISTS transactions
(
    hash    BLOB PRIMARY KEY NOT NULL,
    transaction_proto       BLOB NOT NULL,
    time       INTEGER NOT NULL,
    signable_hash       BLOB NOT NULL,
    first_input_address       BLOB,
    first_output_address       BLOB,
    transaction_type       BLOB NOT NULL,
    total_amount       INTEGER NOT NULL,
    first_output_amount       INTEGER,
    fee_amount       INTEGER,
    remainder_amount       INTEGER,
    contract_type       INTEGER,
    is_test INTEGER NOT NULL,
    is_swap INTEGER NOT NULL,
    is_metadata INTEGER NOT NULL,
    is_request INTEGER NOT NULL,
    is_deploy INTEGER NOT NULL,
    is_liquidity INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS transactions_time_desc
    ON transactions (time DESC );

CREATE INDEX IF NOT EXISTS transactions_time_asc
    ON transactions (time ASC );

CREATE INDEX IF NOT EXISTS transactions_time_desc_is_test
    ON transactions (time DESC, is_test ASC);

CREATE INDEX IF NOT EXISTS transactions_time_hash_desc
    ON transactions (time DESC, hash DESC);

CREATE INDEX IF NOT EXISTS transactions_first_input_address
    ON transactions (first_input_address ASC );

CREATE INDEX IF NOT EXISTS transactions_first_output_address
    ON transactions (first_output_address ASC );

CREATE INDEX IF NOT EXISTS transactions_total_amount
    ON transactions (total_amount DESC);

CREATE INDEX IF NOT EXISTS transactions_first_output_amount
    ON transactions (first_output_amount DESC);

CREATE INDEX IF NOT EXISTS fee_amount
    ON transactions (fee_amount DESC);
