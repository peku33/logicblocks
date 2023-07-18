#[derive(Debug)]
struct DropGuardInner {
    set: bool,
}
impl DropGuardInner {
    pub fn new() -> Self {
        Self { set: false }
    }
    pub fn set(&mut self) {
        assert_eq!(self.set, false, "set twice");
        self.set = true;
    }
}
impl Drop for DropGuardInner {
    fn drop(&mut self) {
        assert_eq!(self.set, true, "never set");
    }
}

#[derive(Debug)]
pub struct DropGuard {
    inner: DropGuardInner,
}
impl DropGuard {
    pub fn new() -> Self {
        let inner = DropGuardInner::new();
        Self { inner }
    }
    pub fn set(mut self) {
        self.inner.set();
    }
}
