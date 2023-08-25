CREATE TABLE IF NOT EXISTS multiparty_signatures (
                                    room_id TEXT PRIMARY KEY,
                                    keygen_room_id TEXT NOT NULL,
                                    proof BLOB NOT NULL,
                                    proof_time INTEGER NOT NULL,
                                    initiate_signing BLOB NOT NULL
);
