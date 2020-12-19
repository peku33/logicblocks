pub mod anyhow_multiple_error;
pub mod async_flag;
pub mod atomic_cell;
pub mod borrowed_async;
pub mod bus;
pub mod erased_ref;
pub mod logging;
pub mod optional_async;
pub mod ready_chunks_dynamic;
pub mod scoped_async;
pub mod select_all_empty;
pub mod sqlite_async;
pub mod waker_stream;

// https://stackoverflow.com/questions/50547766/how-can-i-get-impl-trait-to-use-the-appropriate-lifetime-for-a-mutable-reference
pub trait Captures<'a> {}
impl<'a, T: ?Sized> Captures<'a> for T {}
