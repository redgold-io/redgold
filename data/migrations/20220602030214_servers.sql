CREATE TABLE IF NOT EXISTS servers (
                                    host TEXT PRIMARY KEY NOT NULL,
                                    username TEXT NOT NULL,
                                    key_path TEXT NOT NULL
);