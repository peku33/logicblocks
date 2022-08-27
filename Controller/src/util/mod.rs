pub mod anyhow_multiple_error;
pub mod async_barrier;
pub mod async_ext;
pub mod async_flag;
pub mod fs;
pub mod logging;
pub mod observable;
pub mod runtime;
pub mod waker_stream;

// https://stackoverflow.com/questions/50547766/how-can-i-get-impl-trait-to-use-the-appropriate-lifetime-for-a-mutable-reference
pub trait Captures<'a> {}
impl<'a, T: ?Sized> Captures<'a> for T {}
