use parking_lot::Mutex;
use std::ops::Deref;

#[derive(Debug)]
struct State<V>
where
    V: Clone + Send + Sync + 'static,
{
    value_last: Option<V>,
    user_version: usize,
    device_version: usize,
}
impl<V> State<V>
where
    V: Clone + Send + Sync + 'static,
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
pub struct Property<V>
where
    V: Clone + Send + Sync + 'static,
{
    state: Mutex<State<V>>,
}
impl<V> Property<V>
where
    V: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        let state = State::new();
        let state = Mutex::new(state);

        Self { state }
    }

    // User
    pub fn user_remote(&self) -> Remote<'_, V> {
        Remote::new(self)
    }

    // Device
    pub fn device_pending(&self) -> Option<Pending<'_, V>> {
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
pub struct Remote<'p, V>
where
    V: Clone + Send + Sync + 'static,
{
    property: &'p Property<V>,
}
impl<'p, V> Remote<'p, V>
where
    V: Clone + Send + Sync + 'static,
{
    fn new(property: &'p Property<V>) -> Self {
        Self { property }
    }

    #[must_use = "use this value to wake properties changed waker"]
    pub fn push(
        &self,
        value: V,
    ) -> bool {
        let mut state = self.property.state.lock();

        state.value_last.replace(value);
        state.user_version += 1;

        drop(state);

        true
    }
}

#[derive(Debug)]
pub struct Pending<'p, V: Clone + Send + Sync + 'static> {
    property: &'p Property<V>,
    value: V,
    version: usize,
}
impl<V: Clone + Send + Sync + 'static> Pending<'_, V> {
    pub fn commit(self) {
        let mut state = self.property.state.lock();

        state.device_version = self.version;

        drop(state);
    }
}
impl<V> Deref for Pending<'_, V>
where
    V: Clone + Send + Sync + 'static,
{
    type Target = V;
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
