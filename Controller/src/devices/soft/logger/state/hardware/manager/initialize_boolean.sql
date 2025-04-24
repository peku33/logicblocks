CREATE TABLE IF NOT EXISTS `sinks_ext_boolean` (
    `sink_id` INTEGER REFERENCES `sinks`(`sink_id`) ON DELETE RESTRICT ON UPDATE RESTRICT UNIQUE,
    
    `value_last_timestamp` INTEGER NULL,
    `value_last_value` INTEGER NULL
) STRICT;
CREATE TABLE IF NOT EXISTS `buffer_boolean` (
    `sink_id` INTEGER REFERENCES `sinks`(`sink_id`) ON DELETE RESTRICT ON UPDATE RESTRICT,
    
    `timestamp` INTEGER NOT NULL,
    `value` INTEGER NULL
) STRICT;
CREATE TABLE IF NOT EXISTS `storage_boolean` (
    `sink_id` INTEGER REFERENCES `sinks`(`sink_id`) ON DELETE RESTRICT ON UPDATE RESTRICT,
    `timestamp_group_start` INTEGER NOT NULL, -- FLOOR(timestamp / timestamp_divisor) * timestamp_divisor

    `value_last_timestamp` INTEGER NOT NULL,
    `value_last_value` INTEGER NULL,

    `weight` REAL NOT NULL,
    `sum` INTEGER NOT NULL,
    
    UNIQUE(`sink_id`, `timestamp_group_start`)
) STRICT;
