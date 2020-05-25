pub mod borrowed_async;
pub mod bus;
pub mod bus2;
pub mod select_all_empty;
pub mod sqlite_async;
pub mod tokio_cancelable;

// https://stackoverflow.com/questions/50547766/how-can-i-get-impl-trait-to-use-the-appropriate-lifetime-for-a-mutable-reference
pub trait Captures<'a> {}
impl<'a, T: ?Sized> Captures<'a> for T {}
