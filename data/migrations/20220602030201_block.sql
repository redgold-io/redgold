CREATE TABLE IF NOT EXISTS block (
                                    hash BLOB PRIMARY KEY,
                                    height INTEGER NOT NULL,
                                    raw BLOB NOT NULL,
                                    time INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS block_height
    ON block (height DESC );