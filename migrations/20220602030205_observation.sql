CREATE TABLE IF NOT EXISTS observation (
                                           root    BLOB PRIMARY KEY,
                                           raw_observation    BLOB,
                                           public_key    BLOB,
                                           time INTEGER,
                                           height INTEGER
);