CREATE TABLE IF NOT EXISTS multiparty_signatures (
                                    room_id BLOB PRIMARY KEY NOT NULL,
                                    keygen_room_id BLOB NOT NULL,
                                    proof BLOB NOT NULL,
                                    proof_time INTEGER NOT NULL,
                                    initiate_signing BLOB NOT NULL
);
