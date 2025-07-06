use parking_lot::Mutex;
use std::ops::Deref;

#[derive(Debug)]
struct State<T>
where
    T: Clone + Send + Sync + 'static,
{
    value_last: Option<T>,
    user_version: usize,
    device_version: usize,
}
impl<T> State<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            value_last: None,
            user_version: 0,
            device_version: 0,
        }
    }
}

#[derive(Debug)]
pub struct Property<T>
where
    T: Clone + Send + Sync + 'static,
{
    state: Mutex<State<T>>,
}
impl<T> Property<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        let state = State::new();
        let state = Mutex::new(state);

        Self { state }
    }

    // User
    pub fn user_remote(&self) -> Remote<'_, T> {
        Remote::new(self)
    }

    // Device
    pub fn device_pending(&self) -> Option<Pending<'_, T>> {
        let state = self.state.lock();

        if state.device_version >= state.user_version {
            return None;
        }

        let value = match &state.value_last {
            Some(value) => value.clone(),
            None => return None,
        };

        let pending = Pending {
            property: self,
            value,
            version: state.user_version,
        };

        Some(pending)
    }
}

#[derive(Debug)]
pub struct Remote<'p, T>
where
    T: Clone + Send + Sync + 'static,
{
    property: &'p Property<T>,
}
impl<'p, T> Remote<'p, T>
where
    T: Clone + Send + Sync + 'static,
{
    fn new(property: &'p Property<T>) -> Self {
        Self { property }
    }

    #[must_use = "use this value to wake properties changed waker"]
    pub fn push(
        &self,
        value: T,
    ) -> bool {
        let mut state = self.property.state.lock();

        state.value_last.replace(value);
        state.user_version += 1;

        drop(state);

        true
    }
}

#[derive(Debug)]
pub struct Pending<'p, T: Clone + Send + Sync + 'static> {
    property: &'p Property<T>,
    value: T,
    version: usize,
}
impl<T: Clone + Send + Sync + 'static> Pending<'_, T> {
    pub fn commit(self) {
        let mut state = self.property.state.lock();

        state.device_version = self.version;

        drop(state);
    }
}
impl<T> Deref for Pending<'_, T>
where
    T: Clone + Send + Sync + 'static,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        let property = Property::<usize>::new();
        let sink = property.user_remote();

        // Initial state
        assert!(property.device_pending().is_none());

        // Sink 1
        assert_eq!(sink.push(1usize), true);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 1);
        pending.commit();
        assert!(property.device_pending().is_none());

        // Sink 2
        assert_eq!(sink.push(2usize), true);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 2);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 2);
        pending.commit();
        assert!(property.device_pending().is_none());

        // Sink 3
        assert_eq!(sink.push(3usize), true);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 3);
        assert_eq!(sink.push(4usize), true);
        assert_eq!(*pending, 3);
        pending.commit();
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 4);
        pending.commit();
        assert!(property.device_pending().is_none());
    }
}
