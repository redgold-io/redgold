CREATE TABLE IF NOT EXISTS peer_key (
                                        public_key    BLOB PRIMARY KEY,
                                        id    BLOB,
                                        multi_hash    BLOB,
                                        address    TEXT,
                                        status    TEXT
);