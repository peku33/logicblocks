use parking_lot::Mutex;

#[derive(Debug)]
struct State<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    value: Option<T>,
    user_pending: bool,
}
impl<T> State<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            value: None,
            user_pending: false,
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
    pub fn device_must_read(&self) -> bool {
        let state = self.state.lock();

        let device_must_read = state.value.is_none();

        drop(state);

        device_must_read
    }
    #[must_use = "use this value to wake properties changed waker"]
    pub fn device_set(
        &self,
        value: T,
    ) -> bool {
        let mut state = self.state.lock();

        if state.value.as_ref() == Some(&value) {
            return false;
        }

        state.value.replace(value);
        state.user_pending = true;

        drop(state);

        true
    }
    #[must_use = "use this value to wake properties changed waker"]
    pub fn device_reset(&self) -> bool {
        let mut state = self.state.lock();

        if state.value.is_none() {
            return false;
        }

        state.value = None;
        state.user_pending = true;

        drop(state);

        true
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

    pub fn take_pending(&self) -> Option<Option<T>> {
        let mut state = self.property.state.lock();

        if !state.user_pending {
            return None;
        }

        let value = state.value.clone();
        state.user_pending = false;

        drop(state);

        Some(value)
    }

    pub fn peek_last(&self) -> Option<T> {
        let state = self.property.state.lock();

        let value = state.value.clone();

        drop(state);

        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        let property = Property::<usize>::new();
        let stream = property.user_remote();

        assert!(stream.take_pending().is_none());
        assert!(property.device_must_read());

        assert_eq!(property.device_set(1), true);
        assert_eq!(property.device_set(1), false);
        assert_eq!(stream.take_pending().unwrap().unwrap(), 1);
        assert!(stream.take_pending().is_none());

        assert_eq!(property.device_set(2), true);
        assert_eq!(property.device_set(3), true);
        assert_eq!(stream.take_pending().unwrap().unwrap(), 3);
        assert!(stream.take_pending().is_none());

        assert!(stream.take_pending().is_none());
        assert!(property.device_reset());
        assert!(stream.take_pending().unwrap().is_none());
        assert!(stream.take_pending().is_none());
    }
}
