use super::fs::Fs;
use super::{Context, Module, ModuleFactory};
use crate::util::sqlite_async::SQLiteAsync;
use failure::Error;
use futures::future::Future;
use rusqlite::{Connection, Transaction};

pub struct SQLite {
    sqlite_async: SQLiteAsync,
}
impl SQLite {
    pub fn new(fs: &Fs) -> Self {
        let sqlite_file = fs.persistent_data_directory().join("LogicBlocks.sqlite");

        let sqlite_connection = rusqlite::Connection::open(sqlite_file).unwrap();
        sqlite_connection
            .pragma_update(None, "auto_vacuum", &"INCREMENTAL")
            .unwrap();
        sqlite_connection
            .pragma_update(None, "foreign_keys", &true)
            .unwrap();
        sqlite_connection
            .pragma_update(None, "journal_mode", &"WAL")
            .unwrap();
        sqlite_connection
            .pragma_update(None, "synchronous", &"NORMAL")
            .unwrap();
        let sqlite_async = SQLiteAsync::new(sqlite_connection, "SQLite".to_owned());
        Self { sqlite_async }
    }

    pub fn query<F, R>(
        &self,
        f: F,
    ) -> impl Future<Output = R>
    where
        F: FnOnce(&Connection) -> R + Send + 'static,
        R: Send + 'static,
    {
        self.sqlite_async.query(f)
    }

    pub fn transaction<F, R>(
        &self,
        f: F,
    ) -> impl Future<Output = Result<R, Error>>
    where
        F: FnOnce(&mut Transaction) -> R + Send + 'static,
        R: Send + 'static,
    {
        self.sqlite_async.transaction(f)
    }
}
impl Module for SQLite {}
impl ModuleFactory for SQLite {
    fn spawn(context: &Context) -> Self {
        let fs = context.get::<Fs>();
        Self::new(&fs)
    }
}
