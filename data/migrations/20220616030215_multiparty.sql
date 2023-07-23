CREATE TABLE IF NOT EXISTS multiparty (
                                    room_id TEXT PRIMARY KEY,
                                    local_share TEXT NOT NULL,
                                    keygen_time INTEGER NOT NULL,
                                    initiate_keygen BLOB NOT NULL,
                                    self_initiated INTEGER NOT NULL,
                                    host_public_key BLOB NOT NULL,
                                    keygen_public_key BLOB
);
