use super::fs::Fs;
use anyhow::{Context, Error};
use crossbeam::channel;
use futures::{
    channel::oneshot,
    future::{Future, FutureExt},
};
use rusqlite::{vtab, Connection, Transaction};
use std::{fmt, mem::ManuallyDrop, path::PathBuf, thread};

type Operation = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

#[derive(Debug)]
pub struct SQLite {
    name: String,
    operation_sender: ManuallyDrop<channel::Sender<Operation>>,
    sqlite_thread: ManuallyDrop<thread::JoinHandle<Result<(), Error>>>,
}
impl SQLite {
    pub fn new(
        fs: &Fs,
        name: String,
    ) -> Self {
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_'),
            "database name must be valid for fs path (lower text, digits, dot, underscore)"
        );
        let sqlite_file = fs
            .persistent_data_directory()
            .join([name.as_str(), ".sqlite"].concat());
        let thread_name = format!("{}/sqlite", name);

        let (operation_sender, operation_receiver) = channel::unbounded();
        let operation_sender = ManuallyDrop::new(operation_sender);

        let sqlite_thread = thread::Builder::new()
            .name(thread_name)
            .spawn(|| Self::thread_main(sqlite_file, operation_receiver))
            .unwrap();
        let sqlite_thread = ManuallyDrop::new(sqlite_thread);

        Self {
            name,
            operation_sender,
            sqlite_thread,
        }
    }

    fn thread_main(
        sqlite_file: PathBuf,
        operation_receiver: channel::Receiver<Operation>,
    ) -> Result<(), Error> {
        // initialization
        let mut connection = Connection::open(sqlite_file).context("open")?;
        connection
            .pragma_update(None, "auto_vacuum", &"INCREMENTAL")
            .context("auto_vacuum")?;
        connection
            .pragma_update(None, "foreign_keys", &true)
            .context("foreign_keys")?;
        connection
            .pragma_update(None, "journal_mode", &"WAL")
            .context("journal_mode")?;
        connection
            .pragma_update(None, "synchronous", &"NORMAL")
            .context("synchronous")?;
        vtab::array::load_module(&connection).context("vtab load_module")?;

        // main loop
        while let Ok(operation) = operation_receiver.recv() {
            operation(&mut connection);
        }

        // finalization
        connection
            .close()
            .map_err(|(_, error)| error)
            .context("close")?;

        Ok(())
    }

    pub fn query<E, R>(
        &self,
        e: E,
    ) -> impl Future<Output = R>
    where
        E: FnOnce(&Connection) -> R + Send + 'static,
        R: Send + 'static,
    {
        let (result_sender, result_receiver) = oneshot::channel();
        let operation = Box::new(move |connection: &mut Connection| {
            let result = e(connection);
            let _ = result_sender.send(result);
        });
        self.operation_sender.send(operation).unwrap();
        result_receiver.map(|r| r.unwrap())
    }

    pub fn transaction<E, R>(
        &self,
        e: E,
    ) -> impl Future<Output = Result<R, Error>>
    where
        E: FnOnce(&mut Transaction) -> R + Send + 'static,
        R: Send + 'static,
    {
        let (result_sender, result_receiver) = oneshot::channel();
        let operation = Box::new(move |connection: &mut Connection| {
            let result = try {
                let mut transaction_object = connection.transaction().context("transaction")?;
                let result = e(&mut transaction_object);
                transaction_object.commit().context("commit")?;
                result
            };
            let _ = result_sender.send(result);
        });
        self.operation_sender.send(operation).unwrap();
        result_receiver.map(|r| r.unwrap())
    }
}
impl fmt::Display for SQLite {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "SQlite({})", self.name)
    }
}
impl Drop for SQLite {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.operation_sender) }; // closes channel and exits thread
        unsafe { ManuallyDrop::take(&mut self.sqlite_thread) }
            .join()
            .unwrap()
            .unwrap();
    }
}
