CREATE TABLE IF NOT EXISTS block (
                                    hash BLOB PRIMARY KEY,
                                    height INTEGER,
                                    raw BLOB,
                                    time INTEGER
);

CREATE UNIQUE INDEX IF NOT EXISTS block_height
    ON block (height DESC );