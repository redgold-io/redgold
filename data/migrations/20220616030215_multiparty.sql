CREATE TABLE IF NOT EXISTS multiparty (
                                    room_id TEXT PRIMARY KEY,
                                    local_share TEXT,
                                    keygen_time INTEGER,
                                    initiate_keygen BLOB
);
