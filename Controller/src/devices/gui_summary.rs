use crate::util::async_waker::mpsc;

#[derive(Debug)]
pub struct Waker {
    inner: mpsc::Signal,
}
impl Waker {
    pub fn new() -> Self {
        let inner = mpsc::Signal::new();

        Self { inner }
    }

    pub fn wake(&self) {
        self.inner.wake();
    }

    pub fn as_signal(&self) -> &mpsc::Signal {
        &self.inner
    }
}

pub trait Device {
    fn waker(&self) -> &Waker;

    type Value: erased_serde::Serialize + Send + Sync + 'static;
    fn value(&self) -> Self::Value;
}

pub trait DeviceBase {
    fn waker(&self) -> &Waker;
    fn value(&self) -> Box<dyn erased_serde::Serialize + Send + Sync + 'static>;
}
impl<T: Device> DeviceBase for T {
    fn waker(&self) -> &Waker {
        self.waker()
    }

    fn value(&self) -> Box<dyn erased_serde::Serialize + Send + Sync + 'static> {
        Box::new(self.value())
    }
}
