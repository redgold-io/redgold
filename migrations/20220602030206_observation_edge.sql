CREATE TABLE IF NOT EXISTS observation_edge (
                                                root    BLOB NOT NULL,
                                                leaf_hash    BLOB NOT NULL,
                                                observation_hash    BLOB NOT NULL,
                                                observation_metadata BLOB NOT NULL,
                                                merkle_proof BLOB NOT NULL,
                                                time INTEGER,
                                                PRIMARY KEY(observation_hash, leaf_hash, root)
)