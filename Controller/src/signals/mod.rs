pub mod exchanger;
pub mod signal;
pub mod types;
pub mod waker;

use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    fmt,
    hash::{self},
};

// Identifier
pub trait Identifier: Clone + Eq + hash::Hash + fmt::Debug + Send + Sync + 'static {}

trait IdentifierBase: Send + Sync + 'static {
    fn type_id(&self) -> TypeId;
    fn type_name(&self) -> &str;

    fn as_any(&self) -> &dyn Any;
    fn as_debug(&self) -> &dyn fmt::Debug;

    fn clone(&self) -> Box<dyn IdentifierBase>;
    fn eq(
        &self,
        other: &dyn IdentifierBase,
    ) -> bool;
    fn hash(
        &self,
        state: &mut dyn hash::Hasher,
    );
}
impl<I: Identifier> IdentifierBase for I {
    fn type_id(&self) -> TypeId {
        TypeId::of::<I>()
    }
    fn type_name(&self) -> &str {
        type_name::<I>()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_debug(&self) -> &dyn fmt::Debug {
        self
    }

    fn clone(&self) -> Box<dyn IdentifierBase> {
        let cloned = self.clone();
        let cloned = Box::new(cloned) as Box<dyn IdentifierBase>;
        cloned
    }
    fn eq(
        &self,
        other: &dyn IdentifierBase,
    ) -> bool {
        let other = match other.as_any().downcast_ref::<I>() {
            Some(other) => other,
            None => return false,
        };

        *self == *other
    }
    fn hash(
        &self,
        mut state: &mut dyn hash::Hasher,
    ) {
        // this should prevent similar but different types (like different enums)
        // going into same bucket
        let type_id = <I as IdentifierBase>::type_id(self);
        hash::Hash::hash(&type_id, &mut state);

        // the actual hash
        self.hash(&mut state);
    }
}

pub struct IdentifierBaseWrapper {
    inner: Box<dyn IdentifierBase>,
}
impl IdentifierBaseWrapper {
    pub fn new<I: Identifier>(identifier: I) -> Self {
        let inner = Box::new(identifier) as Box<dyn IdentifierBase>;
        Self { inner }
    }
}
impl Clone for IdentifierBaseWrapper {
    fn clone(&self) -> Self {
        let inner = self.inner.clone();
        Self { inner }
    }
}
impl PartialEq for IdentifierBaseWrapper {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.inner.eq(&*other.inner)
    }
}
impl Eq for IdentifierBaseWrapper {}
impl hash::Hash for IdentifierBaseWrapper {
    fn hash<H: hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.inner.hash(state)
    }
}
impl fmt::Debug for IdentifierBaseWrapper {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_tuple("IdentifierErased")
            .field(&self.inner.type_name())
            .field(self.inner.as_debug())
            .finish()
    }
}

// ByIdentifier
#[allow(type_alias_bounds)] // FIXME: one day maybe this will be fixed
pub type ByIdentifier<'s, I: Identifier> = HashMap<I, &'s dyn signal::Base>;
pub type ByIdentifierBaseWrapper<'s> = HashMap<IdentifierBaseWrapper, &'s dyn signal::Base>;

// Device
pub trait Device: fmt::Debug + Send + Sync {
    fn targets_changed_waker(&self) -> Option<&waker::TargetsChangedWaker>;
    fn sources_changed_waker(&self) -> Option<&waker::SourcesChangedWaker>;

    type Identifier: Identifier;
    fn by_identifier(&self) -> ByIdentifier<Self::Identifier>;
}

pub trait DeviceBase: Send + Sync {
    fn targets_changed_waker(&self) -> Option<&waker::TargetsChangedWaker>;
    fn sources_changed_waker(&self) -> Option<&waker::SourcesChangedWaker>;
    fn by_identifier(&self) -> ByIdentifierBaseWrapper;

    fn type_name(&self) -> &str; // for debugging

    fn as_debug(&self) -> &dyn fmt::Debug;
}
impl<D: Device> DeviceBase for D {
    fn targets_changed_waker(&self) -> Option<&waker::TargetsChangedWaker> {
        self.targets_changed_waker()
    }
    fn sources_changed_waker(&self) -> Option<&waker::SourcesChangedWaker> {
        self.sources_changed_waker()
    }
    fn by_identifier(&self) -> ByIdentifierBaseWrapper {
        self.by_identifier()
            .into_iter()
            .map(|(identifier, signal)| {
                let identifier = IdentifierBaseWrapper::new(identifier);
                (identifier, signal)
            })
            .collect::<ByIdentifierBaseWrapper>()
    }

    fn type_name(&self) -> &str {
        type_name::<D>()
    }

    fn as_debug(&self) -> &dyn fmt::Debug {
        self
    }
}

#[derive(Clone, Copy)]
pub struct DeviceBaseRef<'d> {
    inner: &'d dyn DeviceBase,
}
impl<'d> DeviceBaseRef<'d> {
    pub fn from_device<D: Device>(device: &'d D) -> Self {
        let inner = device as &dyn DeviceBase;
        Self { inner }
    }
    pub fn from_device_base(device_base: &'d dyn DeviceBase) -> Self {
        Self { inner: device_base }
    }

    pub fn targets_changed_waker(&self) -> Option<&'d waker::TargetsChangedWaker> {
        self.inner.targets_changed_waker()
    }
    pub fn sources_changed_waker(&self) -> Option<&'d waker::SourcesChangedWaker> {
        self.inner.sources_changed_waker()
    }
    pub fn by_identifier(&self) -> ByIdentifierBaseWrapper<'d> {
        self.inner.by_identifier()
    }

    pub fn type_name(&self) -> &str {
        self.inner.type_name()
    }
}
impl<'d> fmt::Debug for DeviceBaseRef<'d> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_tuple("DeviceBaseRef")
            .field(&self.inner.type_name())
            .field(self.inner.as_debug())
            .finish()
    }
}
