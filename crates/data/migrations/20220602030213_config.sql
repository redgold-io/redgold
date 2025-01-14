CREATE TABLE IF NOT EXISTS config (
                   key_name     STRING PRIMARY KEY NOT NULL,
                   value_data   STRING,
                   value_bytes BLOB
);