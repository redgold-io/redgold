CREATE TABLE IF NOT EXISTS multiparty_signatures (
                                    room_id TEXT PRIMARY KEY,
                                    keygen_room_id TEXT,
                                    proof BLOB,
                                    proof_time INTEGER,
                                    initiate_signing BLOB
);
