CREATE TABLE IF NOT EXISTS ssh_key (
                   name    STRING PRIMARY KEY,
                   local_path STRING,
                   passphrase STRING,
                   contents STRING
);