use super::async_flag;
use async_trait::async_trait;

#[async_trait]
pub trait Runnable: Send + Sync {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited;
}

#[derive(Debug)]
pub struct Exited;
