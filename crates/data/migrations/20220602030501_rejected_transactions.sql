CREATE TABLE IF NOT EXISTS rejected_transactions(
    hash    BLOB PRIMARY KEY NOT NULL,
    transaction_proto       BLOB NOT NULL,
    time       INTEGER NOT NULL,
    rejection_reason       BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS rejected_transactions_time_desc
    ON rejected_transactions (time DESC );

CREATE INDEX IF NOT EXISTS rejected_transactions_time_asc
    ON rejected_transactions (time ASC );

CREATE INDEX IF NOT EXISTS rejected_transactions_time_hash_desc
    ON rejected_transactions (time DESC, hash DESC);
