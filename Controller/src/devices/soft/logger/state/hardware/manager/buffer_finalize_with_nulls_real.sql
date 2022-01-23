INSERT INTO
    `buffer_real`
    (`sink_id`, `timestamp`, `value`)
SELECT
    `sink_id`, :now, NULL
FROM
    `sinks_ext_real`
;
