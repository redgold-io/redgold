CREATE TABLE IF NOT EXISTS price_time
(
    source INTEGER NOT NULL,
    currency  INTEGER NOT NULL,
    denomination  INTEGER NOT NULL,
    time  INTEGER NOT NULL,
    price REAL NOT NULL,
    PRIMARY KEY (source, currency, denomination, time)
);

CREATE INDEX IF NOT EXISTS price_time_time
    ON price_time (time DESC);

CREATE INDEX IF NOT EXISTS price_time_time_asc
    ON price_time (time ASC);
