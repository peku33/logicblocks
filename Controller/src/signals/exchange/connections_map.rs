use std::{
    collections::{HashMap, HashSet},
    fmt, hash,
};

// One key - multiple values
// One value - one key
#[derive(Debug)]
pub struct ManyFromOne<S, SD, T, TD>
where
    S: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    SD: fmt::Debug,
    T: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    TD: fmt::Debug,
{
    sources: HashMap<S, (SD, HashSet<T>)>,
    targets: HashMap<T, (TD, Option<S>)>,
}
impl<S, SD, T, TD> ManyFromOne<S, SD, T, TD>
where
    S: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    SD: fmt::Debug,
    T: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    TD: fmt::Debug,
{
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            targets: HashMap::new(),
        }
    }

    pub fn insert_source(
        &mut self,
        source: S,
        source_details: SD,
    ) {
        let duplicated = self
            .sources
            .insert(source, (source_details, HashSet::new()))
            .is_some();
        assert!(!duplicated);
    }
    pub fn insert_target(
        &mut self,
        target: T,
        target_details: TD,
    ) {
        let duplicated = self
            .targets
            .insert(target, (target_details, None))
            .is_some();
        assert!(!duplicated);
    }

    pub fn source_details(
        &self,
        source: &S,
    ) -> Option<&SD> {
        self.sources
            .get(source)
            .map(|(source_details, _)| source_details)
    }
    pub fn target_details(
        &self,
        target: &T,
    ) -> Option<&TD> {
        self.targets
            .get(target)
            .map(|(target_details, _)| target_details)
    }

    pub fn set_connections(
        &mut self,
        connections_inverted: HashMap<T, S>,
    ) {
        self.sources
            .values_mut()
            .for_each(|(_, targets)| targets.clear());
        self.targets
            .values_mut()
            .for_each(|(_, source)| *source = None);

        for (target, source) in connections_inverted {
            let inserted = self.sources.get_mut(&source).unwrap().1.insert(target);
            assert!(inserted);

            let duplicated = self
                .targets
                .get_mut(&target)
                .unwrap()
                .1
                .replace(source)
                .is_some();
            assert!(!duplicated)
        }
    }

    pub fn iter_sources(
        &self
    ) -> impl Iterator<Item = ((&S, &SD), impl Iterator<Item = (&T, &TD)>)> {
        self.sources
            .iter()
            .map(move |(source, (source_details, targets))| {
                (
                    (source, source_details),
                    targets
                        .iter()
                        .map(move |target| (target, &self.targets.get(target).unwrap().0)),
                )
            })
    }
    pub fn iter_targets(&self) -> impl Iterator<Item = ((&T, &TD), Option<(&S, &SD)>)> {
        self.targets
            .iter()
            .map(move |(target, (target_details, source))| {
                (
                    (target, target_details),
                    source
                        .as_ref()
                        .map(move |source| (source, &self.sources.get(source).unwrap().0)),
                )
            })
    }
}

#[derive(Debug)]
pub struct ManyFromMany<S, SD, T, TD>
where
    S: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    SD: fmt::Debug,
    T: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    TD: fmt::Debug,
{
    sources: HashMap<S, (SD, HashSet<T>)>,
    targets: HashMap<T, (TD, HashSet<S>)>,
}
impl<S, SD, T, TD> ManyFromMany<S, SD, T, TD>
where
    S: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    SD: fmt::Debug,
    T: Copy + Eq + PartialEq + hash::Hash + fmt::Debug,
    TD: fmt::Debug,
{
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            targets: HashMap::new(),
        }
    }

    pub fn insert_source(
        &mut self,
        source: S,
        source_details: SD,
    ) {
        let duplicated = self
            .sources
            .insert(source, (source_details, HashSet::new()))
            .is_some();
        assert!(!duplicated);
    }
    pub fn insert_target(
        &mut self,
        target: T,
        target_details: TD,
    ) {
        let duplicated = self
            .targets
            .insert(target, (target_details, HashSet::new()))
            .is_some();
        assert!(!duplicated);
    }

    pub fn set_connections(
        &mut self,
        connections: HashMap<S, HashSet<T>>,
    ) {
        self.sources
            .values_mut()
            .for_each(|(_, targets)| targets.clear());
        self.targets
            .values_mut()
            .for_each(|(_, sources)| sources.clear());

        for (source, targets) in connections {
            self.sources
                .get_mut(&source)
                .unwrap()
                .1
                .extend(targets.iter().copied());
            for target in targets {
                let inserted = self.targets.get_mut(&target).unwrap().1.insert(source);
                assert!(inserted);
            }
        }
    }

    pub fn source_details(
        &self,
        source: &S,
    ) -> Option<&SD> {
        self.sources
            .get(source)
            .map(|(source_details, _)| source_details)
    }
    pub fn target_details(
        &self,
        target: &T,
    ) -> Option<&TD> {
        self.targets
            .get(target)
            .map(|(target_details, _)| target_details)
    }

    pub fn iter_sources(
        &self
    ) -> impl Iterator<Item = ((&S, &SD), impl Iterator<Item = (&T, &TD)>)> {
        self.sources
            .iter()
            .map(move |(source, (source_details, targets))| {
                (
                    (source, source_details),
                    targets
                        .iter()
                        .map(move |target| (target, &self.targets.get(target).unwrap().0)),
                )
            })
    }
}
