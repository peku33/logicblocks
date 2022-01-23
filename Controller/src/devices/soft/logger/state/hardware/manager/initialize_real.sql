CREATE TABLE IF NOT EXISTS `sinks_ext_real` (
    `sink_id` REFERENCES `sinks`(`sink_id`) ON DELETE RESTRICT ON UPDATE RESTRICT UNIQUE,
    
    `value_last_timestamp` INTEGER NULL,
    `value_last_value` REAL NULL
);
CREATE TABLE IF NOT EXISTS `buffer_real` (
    `sink_id` REFERENCES `sinks`(`sink_id`) ON DELETE RESTRICT ON UPDATE RESTRICT,
    
    `timestamp` INTEGER NOT NULL,
    `value` REAL NULL
);
CREATE TABLE IF NOT EXISTS `storage_real` (
    `sink_id` REFERENCES `sinks`(`sink_id`) ON DELETE RESTRICT ON UPDATE RESTRICT,
    `timestamp_group_start` INTEGER NOT NULL, -- FLOOR(timestamp / timestamp_divisor) * timestamp_divisor

    `value_last_timestamp` INTEGER NOT NULL,
    `value_last_value` REAL NULL,

    `weight` REAL NOT NULL,
    `sum` REAL NOT NULL,
    `min` REAL NULL,
    `max` REAL NULL,

    UNIQUE(`sink_id`, `timestamp_group_start`)
);
