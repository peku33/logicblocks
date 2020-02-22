use crossbeam_channel::{unbounded, Receiver, Sender};
use failure::Error;
use futures::channel::oneshot;
use futures::{Future, FutureExt};
use rusqlite::{Connection, Transaction};
use std::thread;

type AsyncOperation = Box<dyn FnOnce(&mut Connection) -> () + Send + 'static>;

pub struct SqliteAsync {
    // None after drop
    async_operation_sender: Option<Sender<AsyncOperation>>,

    // None after join
    operation_thread: Option<thread::JoinHandle<()>>,
}
impl SqliteAsync {
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
        loop {
            let async_operation = async_operation_receiver.recv();
            let async_operation = match async_operation {
                Ok(async_operation) => async_operation,
                Err(_) => break,
            };

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
            .clone()
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
            .clone()
            .send(executable)
            .unwrap();
        result_receiver.map(|r| r.unwrap())
    }
}
impl Drop for SqliteAsync {
    fn drop(&mut self) {
        self.async_operation_sender.take().unwrap();
        let _ = self.operation_thread.take().unwrap().join();
    }
}