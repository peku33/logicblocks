use super::fs::FsModule;
use super::{ModuleFactory, ModuleFactoryTrait, ModuleTrait};
use crate::util::sqlite_async::SqliteAsync;
use failure::Error;
use futures::future::{BoxFuture, Future, FutureExt};
use rusqlite::{Connection, Transaction};

pub struct SqliteModule {
    sqlite_async: SqliteAsync,
}
impl SqliteModule {
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
impl ModuleTrait for SqliteModule {}
impl ModuleFactoryTrait for SqliteModule {
    fn spawn<'mf>(module_factory: &'mf ModuleFactory) -> BoxFuture<'mf, Self> {
        async move {
            let fs_module = module_factory.get::<FsModule>().await;

            let sqlite_file = fs_module
                .persistent_data_directory()
                .join("LogicBlocks.sqlite");

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
            let sqlite_async = SqliteAsync::new(sqlite_connection, "SqliteModule".to_owned());
            Self { sqlite_async }
        }
        .boxed()
    }
}
