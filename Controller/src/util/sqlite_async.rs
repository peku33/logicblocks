use anyhow::Error;
use crossbeam::channel::{unbounded, Receiver, Sender};
use futures::{
    channel::oneshot,
    future::{Future, FutureExt},
};
use rusqlite::{Connection, Transaction};
use std::thread;

type AsyncOperation = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

pub struct SQLiteAsync {
    // None after drop
    async_operation_sender: Option<Sender<AsyncOperation>>,

    // None after join
    operation_thread: Option<thread::JoinHandle<()>>,
}
impl SQLiteAsync {
    pub fn new(
        connection: Connection,
        thread_name: String,
    ) -> Self {
        let (async_operation_sender, async_operation_receiver) = unbounded();
        let operation_thread = thread::Builder::new()
            .name(thread_name)
            .spawn(|| Self::operation_thread_main(connection, async_operation_receiver))
            .unwrap();

        Self {
            async_operation_sender: Some(async_operation_sender),
            operation_thread: Some(operation_thread),
        }
    }

    fn operation_thread_main(
        connection: Connection,
        async_operation_receiver: Receiver<AsyncOperation>,
    ) {
        let mut connection = connection;
        while let Ok(async_operation) = async_operation_receiver.recv() {
            async_operation(&mut connection);
        }
    }

    pub fn query<F, R>(
        &self,
        f: F,
    ) -> impl Future<Output = R>
    where
        F: FnOnce(&Connection) -> R + Send + 'static,
        R: Send + 'static,
    {
        let (result_sender, result_receiver) = oneshot::channel();
        let executable = Box::new(move |connection: &mut Connection| {
            let result = f(connection);
            match result_sender.send(result) {
                Ok(()) => (),
                Err(_) => log::warn!("result_sender closed before publishing result"),
            };
        });
        self.async_operation_sender
            .as_ref()
            .unwrap()
            .send(executable)
            .unwrap();
        result_receiver.map(|r| r.unwrap())
    }

    pub fn transaction<F, R>(
        &self,
        f: F,
    ) -> impl Future<Output = Result<R, Error>>
    where
        F: FnOnce(&mut Transaction) -> R + Send + 'static,
        R: Send + 'static,
    {
        let (result_sender, result_receiver) = oneshot::channel();
        let executable = Box::new(move |connection: &mut Connection| {
            let result = try {
                let mut transaction_object = connection.transaction()?;
                let result = f(&mut transaction_object);
                transaction_object.commit()?;
                result
            };
            match result_sender.send(result) {
                Ok(()) => (),
                Err(_) => log::warn!("result_sender closed before publishing result"),
            };
        });
        self.async_operation_sender
            .as_ref()
            .unwrap()
            .send(executable)
            .unwrap();
        result_receiver.map(|r| r.unwrap())
    }
}
impl Drop for SQLiteAsync {
    fn drop(&mut self) {
        self.async_operation_sender.take().unwrap();
        let _ = self.operation_thread.take().unwrap().join();
    }
}
