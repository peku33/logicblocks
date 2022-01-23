INSERT INTO
    `buffer_boolean`
    (`sink_id`, `timestamp`, `value`)
SELECT
    `sink_id`, :now, NULL
FROM
    `sinks_ext_boolean`
;
