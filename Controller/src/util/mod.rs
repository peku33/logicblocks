pub mod anyhow_multiple_error;
pub mod async_flag;
pub mod atomic_cell;
pub mod atomic_cell_erased;
pub mod fs;
pub mod logging;
pub mod optional_async;
pub mod ready_chunks_dynamic;
pub mod runtime;
pub mod waker_stream;

// https://stackoverflow.com/questions/50547766/how-can-i-get-impl-trait-to-use-the-appropriate-lifetime-for-a-mutable-reference
pub trait Captures<'a> {}
impl<'a, T: ?Sized> Captures<'a> for T {}
