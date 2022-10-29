CREATE TABLE IF NOT EXISTS address_block (
                                    address BLOB PRIMARY KEY,
                                    balance INTEGER,
                                    height INTEGER,
                                    hash BLOB
);