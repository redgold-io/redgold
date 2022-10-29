CREATE TABLE IF NOT EXISTS transactions
(
    hash    BLOB PRIMARY KEY,
    raw_transaction       BLOB,
    time       INTEGER
);