use super::types::{Class, TimeValue, Value};
use crate::{
    datatypes::temperature,
    modules::{fs::Fs, sqlite::SQLite},
    util::{
        async_barrier::Barrier,
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use anyhow::{Context, Error, ensure};
use async_trait::async_trait;
use atomic_refcell::AtomicRefCell;
use chrono::{DateTime, Utc};
use crossbeam::channel;
use futures::{
    future::FutureExt,
    select,
    stream::{StreamExt, TryStreamExt},
    try_join,
};
use indoc::indoc;
use std::{
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
    time::Duration,
};

pub type SinkId = usize;
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SinkDataDetails {
    pub name: String,
    pub class: Class,
}
#[derive(Clone, PartialEq, Debug)]
pub struct SinkData {
    pub name: String,
    pub class: Class,           // invariant
    pub timestamp_divisor: f64, // invariant // TODO: move to typed float
    pub enabled: bool,
}

fn sink_data_compatible(
    a: &SinkData,
    b: &SinkData,
) -> bool {
    if a.class != b.class {
        return false;
    }
    if a.timestamp_divisor != b.timestamp_divisor {
        return false;
    }

    true
}

#[derive(Debug)]
pub struct SinkItem {
    pub sink_id: SinkId,
    pub time_value: TimeValue,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DbClass {
    Boolean,
    Real,
}
impl DbClass {
    pub fn from_class(class: Class) -> Self {
        match class {
            Class::Boolean => Self::Boolean,
            Class::Current => Self::Real,
            Class::FlowVolumetric => Self::Real,
            Class::Frequency => Self::Real,
            Class::Multiplier => Self::Real,
            Class::Pressure => Self::Real,
            Class::Ratio => Self::Real,
            Class::Real => Self::Real,
            Class::Resistance => Self::Real,
            Class::Temperature => Self::Real,
            Class::Voltage => Self::Real,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DbValue {
    Boolean(Option<bool>),
    Real(Option<f64>),
}
impl DbValue {
    pub fn from_value(value: Value) -> Self {
        match value {
            Value::Boolean(value) => Self::Boolean(value),
            Value::Current(current) => Self::Real(current.map(|current| current.to_amperes())),
            Value::FlowVolumetric(flow_volumetric) => Self::Real(
                flow_volumetric.map(|flow_volumetric| flow_volumetric.to_cubic_meters_per_second()),
            ),
            Value::Frequency(frequency) => {
                Self::Real(frequency.map(|frequency| frequency.to_hertz()))
            }
            Value::Multiplier(multiplier) => {
                Self::Real(multiplier.map(|multiplier| multiplier.to_f64()))
            }
            Value::Pressure(pressure) => Self::Real(pressure.map(|pressure| pressure.to_pascals())),
            Value::Ratio(value) => Self::Real(value.map(|value| value.to_f64())),
            Value::Real(value) => Self::Real(value.map(|value| value.to_f64())),
            Value::Resistance(resistance) => {
                Self::Real(resistance.map(|resistance| resistance.to_ohms()))
            }
            Value::Temperature(value) => {
                Self::Real(value.map(|value| value.to_unit(temperature::Unit::Kelvin)))
            }
            Value::Voltage(value) => Self::Real(value.map(|value| value.to_volts())),
        }
    }
}

#[derive(Debug)]
pub struct Manager<'f> {
    name: String,

    sqlite: SQLite<'f>,

    initialized: Barrier,

    sink_items_sender: channel::Sender<SinkItem>,
    sink_items_receiver: AtomicRefCell<channel::Receiver<SinkItem>>,
}
impl<'f> Manager<'f> {
    // general
    pub fn new(
        name: String,
        fs: &'f Fs,
    ) -> Self {
        let sqlite = SQLite::new(format!("logger.state.manager.{name}"), fs);

        let initialized = Barrier::new();

        let (sink_items_sender, sink_items_receiver) = channel::unbounded::<SinkItem>();
        let sink_items_receiver = AtomicRefCell::new(sink_items_receiver);

        Self {
            name,

            sqlite,

            initialized,

            sink_items_sender,
            sink_items_receiver,
        }
    }

    // sink accessing
    pub async fn sinks_data_details_get(&self) -> Result<HashMap<SinkId, SinkDataDetails>, Error> {
        self.initialized.waiter().await;

        let rows = self
            .sqlite
            .query(
                |connection| -> Result<_, Error> {
                    let rows = connection
                        .prepare(indoc!("
                            -------------------------------------------------------------------------
                            SELECT
                                `sink_id`, `name`, `class`
                            FROM
                                `sinks`
                            WHERE
                                `enabled`
                        "))?
                        .query_map([], |row| -> rusqlite::Result<(SinkId, String, Class)> {
                            let sink_id = row.get_ref_unwrap(0).as_i64()? as usize;
                            let name = row.get_ref_unwrap(1).as_str()?.to_owned();
                            let class = Class::from_string(row.get_ref_unwrap(2).as_str()?).unwrap();

                            Ok((sink_id, name, class))
                        })?.collect::<rusqlite::Result<Box<[_]>>>()?;

                    Ok(rows)
                },
            )
            .await
            .context("query")?;

        let sink_data = rows
            .into_iter()
            .map(|(sink_id, name, class)| (sink_id, SinkDataDetails { name, class }))
            .collect::<HashMap<_, _>>();

        Ok(sink_data)
    }
    pub async fn sinks_data_set(
        &self,
        mut sinks_data: HashMap<SinkId, SinkData>,
    ) -> Result<(), Error> {
        // FIXME: this should be done atomically, race condition is possible because of
        // read-modify-write pattern

        self.initialized.waiter().await;

        let sinks_data_current = self.db_sinks_data_get().await.context("sinks_data_set")?;

        // remove no longer existing items
        let sink_ids_to_remove = // break
            &(sinks_data_current.keys().copied().collect::<HashSet<_>>()) - // break
            &(sinks_data.keys().copied().collect::<HashSet<_>>());

        if !sink_ids_to_remove.is_empty() {
            self.db_sinks_remove(sink_ids_to_remove)
                .await
                .context("db_sinks_remove")?;
        }

        // don't update identical items
        sinks_data.retain(|sink_id, sink_data| match sinks_data_current.get(sink_id) {
            Some(sink_data_current) => sink_data != sink_data_current,
            None => true,
        });

        // check for upsert collisions
        for (sink_id, sink_data) in &sinks_data {
            let sink_data_current = match sinks_data_current.get(sink_id) {
                Some(sink_data_current) => sink_data_current,
                None => continue,
            };

            ensure!(
                sink_data_compatible(sink_data, sink_data_current),
                "sink #{} - update contains incompatible values",
                sink_id
            );
        }

        // upsert
        if !sinks_data.is_empty() {
            self.db_sinks_upsert(sinks_data)
                .await
                .context("db_sinks_upsert")?;
        }

        Ok(())
    }

    pub fn sink_items_sender_get(&self) -> channel::Sender<SinkItem> {
        self.sink_items_sender.clone()
    }

    // lifecycle methods
    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        const ERROR_DELAY: Duration = Duration::from_secs(5);

        // initialize
        loop {
            match self.initialize_once().await.context("initialize_once") {
                Ok(()) => break,
                Err(error) => {
                    log::error!("{self}: {error:?}");
                    select! {
                        () = tokio::time::sleep(ERROR_DELAY).fuse() => {},
                        () = exit_flag => return Exited,
                    }
                }
            }
        }

        // run
        loop {
            match self.run_once(exit_flag.clone()).await.context("run_once") {
                Ok(Exited) => break,
                Err(error) => {
                    log::error!("{self}: {error:?}");
                    select! {
                        () = tokio::time::sleep(ERROR_DELAY).fuse() => {},
                        () = exit_flag => return Exited,
                    }
                }
            }
        }

        // finalize
        loop {
            match self.finalize_once().await.context("finalize_once") {
                Ok(()) => break,
                Err(error) => {
                    log::error!("{self}: {error:?}");
                    select! {
                        () = tokio::time::sleep(ERROR_DELAY).fuse() => {},
                        () = exit_flag => return Exited,
                    }
                }
            }
        }

        Exited
    }

    async fn initialize_once(&self) -> Result<(), Error> {
        self.db_initialize().await.context("db_initialize")?;

        self.initialized.release();

        Ok(())
    }
    async fn run_once(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Result<Exited, Error> {
        let sink_items_to_buffer_to_storage_runner =
            self.sink_items_to_buffer_to_storage_run(exit_flag);

        let _: (Exited,) = try_join!(sink_items_to_buffer_to_storage_runner).context("try_join")?;

        Ok(Exited)
    }
    async fn finalize_once(&self) -> Result<(), Error> {
        self.db_finalize().await.context("db_finalize")?;

        Ok(())
    }

    // run methods
    const SINK_ITEMS_TO_BUFFER_TO_STORAGE_INTERVAL: Duration = Duration::from_secs(10);
    async fn sink_items_to_buffer_to_storage_run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Result<Exited, Error> {
        tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
            Self::SINK_ITEMS_TO_BUFFER_TO_STORAGE_INTERVAL,
        ))
        .stream_take_until_exhausted(exit_flag)
        .map(Result::<_, Error>::Ok)
        .try_for_each(async |_| {
            self.db_sink_items_to_buffer_to_storage()
                .await
                .context("db_sink_items_to_buffer_to_storage")?;

            Ok(())
        })
        .await?;

        Ok(Exited)
    }

    // db methods
    async fn db_initialize(&self) -> Result<(), Error> {
        self.sqlite
            .transaction(|transaction| -> Result<(), Error> {
                Self::sql_initialize(transaction).context("sql_initialize")?;
                Self::sql_buffer_to_storage(transaction).context("sql_buffer_to_storage")?;

                Ok(())
            })
            .await
            .context("transaction")??;

        Ok(())
    }
    async fn db_sinks_data_get(&self) -> Result<HashMap<SinkId, SinkData>, Error> {
        let sinks_data = self
            .sqlite
            .query(|connection| -> Result<_, Error> {
                let rows = connection
                    .prepare(indoc!("
                        -----------------------------------------------------------------------------
                        SELECT
                            `sink_id`, `name`, `class`, `timestamp_divisor`, `enabled`
                        FROM
                            `sinks`
                    "
                    ))?
                    .query_map(
                        [],
                        |row| -> rusqlite::Result<(SinkId, String, Class, f64, bool)> {
                            let sink_id = row.get_ref_unwrap(0).as_i64()? as usize;
                            let name = row.get_ref_unwrap(1).as_str()?.to_owned();
                            let class =
                                Class::from_string(row.get_ref_unwrap(2).as_str()?).unwrap();
                            let timestamp_divisor = row.get_ref_unwrap(3).as_f64()?;
                            let enabled = row.get_ref_unwrap(4).as_i64()? != 0;

                            Ok((sink_id, name, class, timestamp_divisor, enabled))
                        },
                    )?
                    .collect::<rusqlite::Result<Box<[_]>>>()?;

                Ok(rows)
            })
            .await
            .context("query")?;

        let sinks_data = sinks_data
            .into_iter()
            .map(|(sink_id, name, class, timestamp_divisor, enabled)| {
                let sink_data = SinkData {
                    name,
                    class,
                    timestamp_divisor,
                    enabled,
                };
                (sink_id, sink_data)
            })
            .collect::<HashMap<_, _>>();

        Ok(sinks_data)
    }
    async fn db_sinks_remove(
        &self,
        sink_ids: HashSet<SinkId>,
    ) -> Result<(), Error> {
        if sink_ids.is_empty() {
            return Ok(());
        }

        self.sqlite
            .transaction(|connection| -> Result<_, Error> {
                Self::sql_sinks_remove(connection, sink_ids).context("sql_sinks_remove")?;

                Ok(())
            })
            .await
            .context("transaction")??;

        Ok(())
    }
    async fn db_sinks_upsert(
        &self,
        sinks_data: HashMap<SinkId, SinkData>,
    ) -> Result<(), Error> {
        if sinks_data.is_empty() {
            return Ok(());
        }

        self.sqlite
            .transaction(|connection| -> Result<_, Error> {
                Self::sql_sinks_upsert(connection, sinks_data).context("sql_sinks_upsert")?;

                Ok(())
            })
            .await
            .context("transaction")??;

        Ok(())
    }
    async fn db_sink_items_to_buffer_to_storage(&self) -> Result<(), Error> {
        let sink_items_receiver = self.sink_items_receiver.borrow();

        if sink_items_receiver.is_empty() {
            return Ok(());
        }

        // split by storage type
        let mut items_boolean = Vec::<(SinkId, DateTime<Utc>, Option<bool>)>::new();
        let mut items_real = Vec::<(SinkId, DateTime<Utc>, Option<f64>)>::new();

        while let Ok(sink_item) = sink_items_receiver.try_recv() {
            let SinkItem {
                sink_id,
                time_value: TimeValue { time, value },
            } = sink_item;

            let value = DbValue::from_value(value);

            match value {
                DbValue::Boolean(value) => items_boolean.push((sink_id, time, value)),
                DbValue::Real(value) => items_real.push((sink_id, time, value)),
            }
        }

        let mut sink_any = false;

        // boolean
        if !items_boolean.is_empty() {
            self.sqlite.transaction(|transaction| -> Result<(), Error> {
                let mut statement = transaction
                    .prepare(indoc!("
                        ---------------------------------------------------------------------------------
                        INSERT INTO
                            `buffer_boolean` (`sink_id`, `timestamp`, `value`)
                        VALUES
                            (?, ?, ?)
                    "))
                    .context("prepare")?;

                for (sink_id, time, value) in items_boolean {
                    let params = rusqlite::params![
                        sink_id as i64,
                        time.timestamp(),
                        value,
                    ];

                    statement.insert(params).context("execute")?;
                }

                Ok(())
            }).await.context("transaction")??;

            sink_any = true;
        }

        // real
        if !items_real.is_empty() {
            self.sqlite.transaction(|transaction| -> Result<(), Error> {
                let mut statement = transaction
                    .prepare(indoc!("
                        ---------------------------------------------------------------------------------
                        INSERT INTO
                            `buffer_real` (`sink_id`, `timestamp`, `value`)
                        VALUES
                            (?, ?, ?)
                    "))
                    .context("prepare")?;

                for (sink_id, time, value) in items_real {
                    let params = rusqlite::params![
                        sink_id as i64,
                        time.timestamp(),
                        value,
                    ];

                    statement.insert(params).context("execute")?;
                }

                Ok(())
            }).await.context("transaction")??;

            sink_any = true;
        }

        // forward all
        if sink_any {
            self.sqlite
                .transaction(|transaction| -> Result<(), Error> {
                    Self::sql_buffer_to_storage(transaction).context("sql_buffer_to_storage")?;

                    Ok(())
                })
                .await
                .context("transaction")??;
        }

        Ok(())
    }
    async fn db_finalize(&self) -> Result<(), Error> {
        self.sqlite
            .transaction(|transaction| -> Result<(), Error> {
                Self::sql_buffer_finalize_with_nulls(transaction)
                    .context("sql_buffer_finalize_with_nulls")?;

                Self::sql_buffer_to_storage(transaction) // break
                    .context("sql_buffer_to_storage")?;

                Ok(())
            })
            .await
            .context("transaction")??;

        Ok(())
    }

    // sql wrappers
    fn sql_initialize(transaction: &rusqlite::Transaction) -> Result<(), Error> {
        // creates the tables

        transaction
            .execute_batch(include_str!("initialize.sql"))
            .context("initialize")?;

        transaction
            .execute_batch(include_str!("initialize_boolean.sql"))
            .context("initialize_boolean")?;
        transaction
            .execute_batch(include_str!("initialize_real.sql"))
            .context("initialize_real")?;

        Ok(())
    }
    fn sql_sinks_remove(
        transaction: &rusqlite::Transaction,
        sink_ids: HashSet<SinkId>,
    ) -> Result<(), Error> {
        if sink_ids.is_empty() {
            return Ok(());
        }

        let sink_ids = sink_ids
            .into_iter()
            .map(|sink_id| rusqlite::types::Value::Integer(sink_id as i64))
            .collect::<Vec<_>>();
        let sink_ids = Rc::new(sink_ids);

        let params = rusqlite::named_params! {
            ":sink_ids": sink_ids,
        };

        // storage
        transaction
            .execute(
                indoc!("
                    ---------------------------------------------------------------------------------
                    DELETE FROM
                        `storage_boolean`
                    WHERE
                        `sink_id` IN rarray(:sink_ids)
                "),
                params,
            )
            .context("execute")?;

        transaction
            .execute(
                indoc!("
                    ---------------------------------------------------------------------------------
                    DELETE FROM
                        `storage_real`
                    WHERE
                        `sink_id` IN rarray(:sink_ids)
                "),
                params,
            )
            .context("execute")?;

        // buffer
        transaction
            .execute(
                indoc!("
                    ---------------------------------------------------------------------------------
                    DELETE FROM
                        `buffer_boolean`
                    WHERE
                        `sink_id` IN rarray(:sink_ids)
                "),
                params,
            )
            .context("execute")?;

        transaction
            .execute(
                indoc!("
                    ---------------------------------------------------------------------------------
                    DELETE FROM
                        `buffer_real`
                    WHERE
                        `sink_id` IN rarray(:sink_ids)
                "),
                params,
            )
            .context("execute")?;

        // sink_ext
        transaction
            .execute(
                indoc!("
                    ---------------------------------------------------------------------------------
                    DELETE FROM
                        `sinks_ext_boolean`
                    WHERE
                        `sink_id` IN rarray(:sink_ids)
                "),
                params,
            )
            .context("execute")?;

        transaction
            .execute(
                indoc!("
                    ---------------------------------------------------------------------------------
                    DELETE FROM
                        `sinks_ext_real`
                    WHERE
                        `sink_id` IN rarray(:sink_ids)
                "),
                params,
            )
            .context("execute")?;

        // sinks
        transaction
            .execute(
                indoc!("
                    ---------------------------------------------------------------------------------
                    DELETE FROM
                        `sinks`
                    WHERE
                        `sink_id` IN rarray(:sink_ids)
                "),
                params,
            )
            .context("execute")?;

        Ok(())
    }
    fn sql_sinks_upsert(
        transaction: &rusqlite::Transaction,
        sinks_data: HashMap<SinkId, SinkData>,
    ) -> Result<(), Error> {
        if sinks_data.is_empty() {
            return Ok(());
        }

        // sinks
        let mut query = transaction
            .prepare(indoc!("
                -------------------------------------------------------------------------------------
                INSERT INTO
                    `sinks`
                    (`sink_id`, `name`, `class`, `timestamp_divisor`, `enabled`)
                VALUES
                    (:sink_id, :name, :class, :timestamp_divisor, :enabled)
                ON CONFLICT
                    (`sink_id`)
                DO UPDATE SET
                    `name` = EXCLUDED.`name`,
                    `enabled` = EXCLUDED.`enabled`
            "))
            .context("prepare")?;

        for (sink_id, sink_data) in &sinks_data {
            let params = rusqlite::named_params! {
                ":sink_id": *sink_id as i64,
                ":name": sink_data.name,
                ":class": sink_data.class.to_string(),
                ":timestamp_divisor": sink_data.timestamp_divisor,
                ":enabled": sink_data.enabled,
            };
            query.execute(params).context("execute")?;
        }

        // sinks_ext
        let sink_ids_boolean = sinks_data
            .iter()
            .filter_map(|(sink_id, sink_data)| {
                if DbClass::from_class(sink_data.class) == DbClass::Boolean {
                    Some(sink_id)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();
        if !sink_ids_boolean.is_empty() {
            let mut query = transaction
                .prepare(indoc!("
                    ---------------------------------------------------------------------------------
                    INSERT INTO
                        `sinks_ext_boolean`
                        (`sink_id`, `value_last_timestamp`, `value_last_value`)
                    VALUES
                        (:sink_id, NULL, NULL)
                    ON CONFLICT
                        (`sink_id`)
                    DO NOTHING
                "))
                .context("prepare")?;

            for sink_id in sink_ids_boolean {
                let params = rusqlite::named_params! {
                    ":sink_id": *sink_id as i64,
                };
                query.execute(params).context("execute")?;
            }
        }

        let sink_ids_real = sinks_data
            .iter()
            .filter_map(|(sink_id, sink_data)| {
                if DbClass::from_class(sink_data.class) == DbClass::Real {
                    Some(sink_id)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();
        if !sink_ids_real.is_empty() {
            let mut query = transaction
                .prepare(indoc!("
                    ---------------------------------------------------------------------------------
                    INSERT INTO
                        `sinks_ext_real`
                        (`sink_id`, `value_last_timestamp`, `value_last_value`)
                    VALUES
                        (:sink_id, NULL, NULL)
                    ON CONFLICT
                        (`sink_id`)
                    DO NOTHING
                "))
                .context("prepare")?;

            for sink_id in sink_ids_real {
                let params = rusqlite::named_params! {
                    ":sink_id": *sink_id as i64,
                };
                query.execute(params).context("execute")?;
            }
        }

        Ok(())
    }
    fn sql_buffer_finalize_with_nulls(transaction: &rusqlite::Transaction) -> Result<(), Error> {
        // appends "null" value to each buffer at current time point

        let params = rusqlite::named_params! {
            ":now": Utc::now().timestamp(),
        };

        transaction
            .execute(
                include_str!("buffer_finalize_with_nulls_boolean.sql"),
                params,
            )
            .context("buffer_finalize_with_nulls_boolean")?;

        transaction
            .execute(
                include_str!("buffer_finalize_with_nulls_real.sql"), // break
                params,
            )
            .context("buffer_finalize_with_nulls_real")?;

        Ok(())
    }
    fn sql_buffer_to_storage(transaction: &rusqlite::Transaction) -> Result<(), Error> {
        transaction
            .execute_batch(include_str!("buffer_to_storage_boolean.sql"))
            .context("buffer_to_storage_boolean")?;

        transaction
            .execute_batch(include_str!("buffer_to_storage_real.sql"))
            .context("buffer_to_storage_real")?;

        Ok(())
    }
}
#[async_trait]
impl Runnable for Manager<'_> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}
impl fmt::Display for Manager<'_> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "Manager({})", self.name)
    }
}
