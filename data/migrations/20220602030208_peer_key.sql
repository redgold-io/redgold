CREATE TABLE IF NOT EXISTS peer_key (
                                        public_key    BLOB PRIMARY KEY,
                                        id    BLOB,
                                        multi_hash    BLOB,
                                        address    TEXT,
                                        status    TEXT,
                                        last_seen    INTEGER DEFAULT 0,
                                        tx BLOB,
                                        node_metadata BLOB,
                                        peer_node_info BLOB
);