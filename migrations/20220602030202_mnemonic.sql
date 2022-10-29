CREATE TABLE IF NOT EXISTS mnemonic (
                   words    STRING PRIMARY KEY,
                   time INTEGER,
                   peer_id BLOB
);