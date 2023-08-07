CREATE TABLE IF NOT EXISTS nodes (
                                        public_key    BLOB PRIMARY KEY,
                                        peer_id    BLOB NOT NULL,
                                        status    TEXT NOT NULL,
                                        last_seen    INTEGER DEFAULT 0,
                                        tx BLOB NOT NULL
);