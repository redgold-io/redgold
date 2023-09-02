CREATE TABLE IF NOT EXISTS nodes (
                                        public_key    BLOB PRIMARY KEY NOT NULL,
                                        peer_id    BLOB NOT NULL,
                                        status    TEXT NOT NULL,
                                        last_seen    INTEGER DEFAULT 0,
                                        tx BLOB NOT NULL
);


CREATE INDEX IF NOT EXISTS nodes_peer_id
    ON nodes (peer_id DESC);

CREATE INDEX IF NOT EXISTS nodes_last_seen
    ON nodes (last_seen DESC);
