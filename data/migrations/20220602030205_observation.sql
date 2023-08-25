CREATE TABLE IF NOT EXISTS observation (
                                           hash    BLOB PRIMARY KEY,
                                           raw    BLOB NOT NULL,
                                           public_key    BLOB NOT NULL,
                                           time INTEGER NOT NULL,
                                           height INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS observation_height
    ON observation (height DESC );

CREATE INDEX IF NOT EXISTS observation_time
    ON observation (time DESC );

CREATE INDEX IF NOT EXISTS observation_key
    ON observation (public_key ASC);
