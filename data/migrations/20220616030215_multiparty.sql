CREATE TABLE IF NOT EXISTS multiparty (
                                    room_id BLOB PRIMARY KEY NOT NULL,
                                    keygen_time INTEGER NOT NULL,
                                    party_info BLOB NOT NULL,
                                    self_initiated INTEGER NOT NULL,
                                    host_public_key BLOB NOT NULL,
                                    keygen_public_key BLOB,
                                    party_data BLOB
);

CREATE INDEX multiparty_time ON multiparty(keygen_time DESC);
CREATE INDEX multiparty_host_public_key ON multiparty(host_public_key ASC);
CREATE INDEX multiparty_keygen_public_key ON multiparty(keygen_public_key ASC);