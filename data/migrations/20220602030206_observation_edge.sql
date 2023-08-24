CREATE TABLE IF NOT EXISTS observation_edge (
                                                root    BLOB NOT NULL,
                                                leaf_hash    BLOB NOT NULL,
                                                observation_hash    BLOB NOT NULL,
                                                observed_hash BLOB NOT NULL,
                                                edge BLOB NOT NULL,
                                                time INTEGER NOT NULL,
                                                PRIMARY KEY(observation_hash, leaf_hash, root, observed_hash)
);

CREATE UNIQUE INDEX IF NOT EXISTS observation_edge_observed_hash
    ON observation_edge (observed_hash DESC);
