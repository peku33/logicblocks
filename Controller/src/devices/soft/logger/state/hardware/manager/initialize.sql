CREATE TABLE IF NOT EXISTS `sinks` (
    `sink_id` INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    `name` TEXT NOT NULL,
    `class` TEXT NOT NULL,
    `timestamp_divisor` REAL NOT NULL,
    `enabled` INTEGER NOT NULL
) STRICT;
