CREATE TABLE IF NOT EXISTS transactions
(
    hash    BLOB PRIMARY KEY,
    raw       BLOB NOT NULL,
    time       INTEGER NOT NULL,
    rejection_reason       BLOB,
    accepted INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS transaction_time_desc
    ON transactions (time DESC );
