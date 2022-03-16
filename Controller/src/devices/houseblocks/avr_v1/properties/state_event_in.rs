use parking_lot::Mutex;
use std::mem::replace;

#[derive(Debug)]
struct State<S, E>
where
    S: Eq + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    state: Option<S>,
    events: Vec<E>,
    user_pending: bool,
}
impl<S, E> State<S, E>
where
    S: Eq + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            state: None,
            events: Vec::<E>::new(),
            user_pending: false,
        }
    }
}

#[derive(Debug)]
pub struct Property<S, E>
where
    S: Eq + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    state: Mutex<State<S, E>>,
}
impl<S, E> Property<S, E>
where
    S: Eq + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        let state = State::new();
        let state = Mutex::new(state);

        Self { state }
    }

    // User
    pub fn user_remote(&self) -> Remote<S, E> {
        Remote::new(self)
    }

    // Device
    pub fn device_must_read(&self) -> bool {
        let inner_state = self.state.lock();

        let device_must_read = inner_state.state.is_none();

        drop(inner_state);

        device_must_read
    }
    #[must_use = "use this value to wake properties changed waker"]
    pub fn device_set(
        &self,
        state: S,
        event: E,
    ) -> bool {
        let mut inner_state = self.state.lock();

        inner_state.state.replace(state);
        inner_state.events.push(event);
        inner_state.user_pending = true;

        drop(inner_state);

        true
    }
    #[must_use = "use this value to wake properties changed waker"]
    pub fn device_reset(&self) -> bool {
        let mut inner_state = self.state.lock();

        inner_state.state = None;
        inner_state.events.clear();
        inner_state.user_pending = true;

        drop(inner_state);

        true
    }
}

#[derive(Debug)]
pub struct Remote<'p, S, E>
where
    S: Eq + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    property: &'p Property<S, E>,
}
impl<'p, S, E> Remote<'p, S, E>
where
    S: Eq + Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn new(property: &'p Property<S, E>) -> Self {
        Self { property }
    }

    pub fn take_pending(&self) -> Option<(Option<S>, Box<[E]>)> {
        let mut state_inner = self.property.state.lock();

        if !state_inner.user_pending {
            return None;
        }

        let state = state_inner.state.clone();
        let events = replace(&mut state_inner.events, Vec::<E>::new()).into_boxed_slice();
        state_inner.user_pending = false;

        drop(state_inner);

        Some((state, events))
    }

    pub fn peek_last(&self) -> Option<S> {
        let state_inner = self.property.state.lock();

        let value = state_inner.state.clone();

        drop(state_inner);

        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        let property = Property::<[bool; 2], [u8; 2]>::new();
        let stream = property.user_remote();

        assert!(stream.take_pending().is_none());
        assert!(property.device_must_read());

        assert_eq!(property.device_set([false, false], [1, 2]), true);
        assert_eq!(
            stream.take_pending().unwrap(),
            (Some([false, false]), vec![[1, 2]].into_boxed_slice())
        );
        assert!(stream.take_pending().is_none());

        assert!(stream.take_pending().is_none());
        assert!(property.device_reset());
        assert_eq!(
            stream.take_pending().unwrap(),
            (None, vec![].into_boxed_slice())
        );
        assert!(stream.take_pending().is_none());
    }
}
