WITH
    `t1` AS (
        SELECT
            `buffer_boolean`.ROWID AS `rowid_`
        FROM
            `buffer_boolean`
        JOIN
            `sinks_ext_boolean` USING(`sink_id`)
        WHERE
            `timestamp` <= `value_last_timestamp`
    )
DELETE FROM
    `buffer_boolean`
WHERE
    ROWID IN (SELECT `rowid_` FROM `t1`)
;

WITH
    `t1` AS (
        SELECT
            `sink_id`,
            `value_last_timestamp` AS `timestamp`,
            `value_last_value` AS `value`
        FROM
            `sinks_ext_boolean`
        WHERE
            `value_last_timestamp` IS NOT NULL

        UNION ALL

        SELECT
            `sink_id`,
            `timestamp`,
            `value`
        FROM
            `buffer_boolean`
    ),
    `t2` AS (
        SELECT
            `sink_id`,
            `timestamp` AS `timestamp_start`,
            LEAD(`timestamp`) OVER(PARTITION BY `sink_id` ORDER BY `timestamp`) AS `timestamp_end`,
            CAST(`timestamp` / `timestamp_divisor` AS INTEGER) * `timestamp_divisor` AS `timestamp_group_start`,
            `value`
        FROM
            `t1`
        JOIN
            `sinks` USING(`sink_id`)
    ),
    `t3` AS (
        SELECT
            `sink_id`,
            `timestamp_group_start`,

            LAST_VALUE(`timestamp_start`) OVER(PARTITION BY `sink_id`, `timestamp_group_start` ORDER BY `timestamp_start` DESC) AS `value_last_timestamp`,
            LAST_VALUE(`value`) OVER(PARTITION BY `sink_id`, `timestamp_group_start` ORDER BY `timestamp_start` DESC) AS `value_last_value`,

            SUM(IIF(`value` IS NOT NULL, `timestamp_end` - `timestamp_start`, 0)) AS `weight`,
            SUM(IIF(`value` IS NOT NULL, (`timestamp_end` - `timestamp_start`) * `value`, 0)) AS `sum`
        FROM
            `t2`
        WHERE
            `timestamp_end` IS NOT NULL
        GROUP BY
            `sink_id`,
            `timestamp_group_start`
    )
INSERT INTO
    `storage_boolean`
    (`sink_id`, `timestamp_group_start`, `value_last_timestamp`, `value_last_value`, `weight`, `sum`)
SELECT
    `sink_id`, `timestamp_group_start`, `value_last_timestamp`, `value_last_value`, `weight`, `sum`
FROM
    `t3`
WHERE
    TRUE
ON CONFLICT
    (`sink_id`, `timestamp_group_start`)
DO UPDATE SET
    `value_last_timestamp` = IIF(EXCLUDED.`value_last_timestamp` > `value_last_timestamp`, EXCLUDED.`value_last_timestamp`, `value_last_timestamp`),
    `value_last_value` = IIF(EXCLUDED.`value_last_timestamp` > `value_last_timestamp`, EXCLUDED.`value_last_value`, `value_last_value`),
    `weight` = `weight` + EXCLUDED.`weight`,
    `sum` = `sum` + EXCLUDED.`sum`
;

WITH
    `t1` AS (
        SELECT
            `sink_id`,
            `timestamp`,
            `value`,
            ROW_NUMBER() OVER(
                PARTITION BY
                    `sink_id`
                ORDER BY
                    `timestamp` DESC
            ) AS `row_number_`
        FROM
            `buffer_boolean`
    ),
    `t2` AS (
        SELECT
            `sink_id`,
            `timestamp`,
            `value`
        FROM
            `t1`
        WHERE
            `row_number_` = 1
    )
UPDATE
    `sinks_ext_boolean`
SET
    `value_last_timestamp` = `timestamp`,
    `value_last_value` = `value`
FROM
    `t2`
WHERE
    `sinks_ext_boolean`.`sink_id` = `t2`.`sink_id`
;

DELETE FROM
    `buffer_boolean`
;
