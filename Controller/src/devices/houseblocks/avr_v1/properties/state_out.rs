use parking_lot::Mutex;
use std::ops::Deref;

#[derive(Debug)]
struct State<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    value: T,
    device_pending: bool,
}
impl<T> State<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    pub fn new(initial: T) -> Self {
        Self {
            value: initial,
            device_pending: true,
        }
    }
}

#[derive(Debug)]
pub struct Property<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    state: Mutex<State<T>>,
}
impl<T> Property<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    pub fn new(initial: T) -> Self {
        let state = State::new(initial);
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

        if !state.device_pending {
            return None;
        }

        let pending = Pending {
            property: self,
            value: state.value.clone(),
        };

        drop(state);

        Some(pending)
    }
    pub fn device_reset(&self) {
        let mut state = self.state.lock();

        state.device_pending = true;

        drop(state);
    }
}

#[derive(Debug)]
pub struct Remote<'p, T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    property: &'p Property<T>,
}
impl<'p, T> Remote<'p, T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    fn new(property: &'p Property<T>) -> Self {
        Self { property }
    }

    #[must_use = "use this value to wake properties changed waker"]
    pub fn set(
        &self,
        value: T,
    ) -> bool {
        let mut state = self.property.state.lock();

        if state.value == value {
            return false;
        }

        state.value = value;
        state.device_pending = true;

        drop(state);

        true
    }

    pub fn peek_last(&self) -> T {
        let state = self.property.state.lock();

        let value = state.value.clone();

        drop(state);

        value
    }
}

#[derive(Debug)]
pub struct Pending<'p, T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    property: &'p Property<T>,
    value: T,
}
impl<T> Pending<'_, T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    pub fn commit(self) {
        let mut lock = self.property.state.lock();

        if lock.value == self.value {
            lock.device_pending = false;
        }
    }
}
impl<T> Deref for Pending<'_, T>
where
    T: Eq + Clone + Send + Sync + 'static,
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
        let property = Property::new(1usize);
        let remote = property.user_remote();

        // Initial value, no commit
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 1);
        assert!(property.device_pending().is_some());

        // Initial value, commit
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 1);
        pending.commit();
        assert!(property.device_pending().is_none());

        // No change
        assert_eq!(remote.set(1), false);
        assert!(property.device_pending().is_none());

        // Change
        assert_eq!(remote.set(2), true);
        assert_eq!(remote.set(2), false);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 2);
        pending.commit();
        assert!(property.device_pending().is_none());

        // Two changes
        assert_eq!(remote.set(3), true);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 3);
        assert_eq!(remote.set(4), true);
        assert_eq!(*pending, 3);
        pending.commit();

        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 4);
        pending.commit();
        assert!(property.device_pending().is_none());
    }
}
