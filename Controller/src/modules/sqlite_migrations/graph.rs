use super::{Migration, Resolver, Version};
use itertools::Itertools;
use phf::Map;

pub type Graph = Map<Version, Map<Version, Option<Migration>>>; // {target => {source => migration}}

pub struct GraphResolver<'g>(pub &'g Graph);
impl<'g> Resolver for GraphResolver<'g> {
    fn resolve(
        &self,
        current: Version,
    ) -> (Version, Option<Box<[Migration]>>) {
        resolve(self.0, current)
    }
}

pub fn resolve(
    graph: &Graph,
    source: Version,
) -> (Version, Option<Box<[Migration]>>) {
    // validate graph. errors in graph are treated as programming error and cannot be handled
    graph.into_iter().for_each(|(target, sources)| {
        sources.into_iter().for_each(|(source, _)| {
            // only forward migrations are supported for now
            // this also implies that target > 0 and that graph contains no cycles and is monotonic
            assert!(target > source);
        });
    });

    // calculate target version as maximum key
    let target = match graph.keys().max() {
        Some(target_version) => *target_version,
        None => 0,
    };

    // only forward migrations are supported
    if target < source {
        return (target, None);
    }

    // if version matches - return no migrations
    if target == source {
        return (target, Some(Vec::new().into_boxed_slice()));
    }

    // call recursive steps
    let path = resolve_step(graph, source, target, Vec::new());

    // recreate path
    let migrations = path.map(|path| {
        path.array_windows::<2>() // pairwise
            .rev() // the path is target -> source, we need source -> target
            .filter_map(|[target, source]| *graph.get(target).unwrap().get(source).unwrap()) // resolve sql, only if defined
            .collect::<Vec<_>>()
            .into_boxed_slice()
    });

    (target, migrations)
}
fn resolve_step(
    graph: &Graph,
    search: Version,        // node we are looking for
    current: Version,       // current recursive step
    mut path: Vec<Version>, // path from source
) -> Option<Vec<Version>> {
    // add current node to path
    path.push(current);

    // check for exit condition
    if search == current {
        return Some(path);
    }

    // apply search
    graph
        .get(&current)
        .unwrap()
        .keys()
        .copied() // all sources for current node
        .sorted() // since the graph is monotonic and we are looking in descending order (search >= current), we order possible nodes lowest to highest to make as low number of steps as possible
        .filter(|source| *source >= search) // we filter out impossible candidates (non-monotonic)
        .find_map(|source| resolve_step(graph, search, source, path.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use phf::phf_map;

    #[test]
    fn resolve_simple_forward() {
        let graph: Graph = phf_map! {
            1u32 => phf_map!{
                0u32 => Some("0to1"),
            },
        };

        let (target, migrations) = resolve(&graph, 0);
        assert_eq!(target, 1);
        assert_eq!(migrations, Some(vec!["0to1"].into_boxed_slice()));
    }
    #[test]
    fn resolve_current_match() {
        let graph: Graph = phf_map! {
            1u32 => phf_map!{
                0u32 => Some("0to1"),
            },
        };

        let (target, migrations) = resolve(&graph, 1);
        assert_eq!(target, 1);
        assert_eq!(migrations, Some(Vec::new().into_boxed_slice()));
    }
    #[test]
    fn resolve_missing_path() {
        let graph: Graph = phf_map! {
            1u32 => phf_map! {},
        };

        let (target, migrations) = resolve(&graph, 0);
        assert_eq!(target, 1);
        assert_eq!(migrations, None);
    }
    #[test]
    fn resolve_noop_migration() {
        let graph: Graph = phf_map! {
            1u32 => phf_map!{
                0u32 => Some("0to1"),
            },
            2u32 => phf_map!{
                1u32 => None,
            },
        };

        let (target, migrations) = resolve(&graph, 0);
        assert_eq!(target, 2);
        assert_eq!(migrations, Some(vec!["0to1"].into_boxed_slice()));
    }
    #[test]
    fn resolve_noop_path() {
        let graph: Graph = phf_map! {
            1u32 => phf_map!{
                0u32 => None,
            },
        };

        let (target, migrations) = resolve(&graph, 0);
        assert_eq!(target, 1);
        assert_eq!(migrations, Some(Vec::new().into_boxed_slice()));
    }
    #[test]
    fn resolve_short_path_1() {
        let graph: Graph = phf_map! {
            1u32 => phf_map!{
                0u32 => Some("0to1"),
            },
            2u32 => phf_map!{
                1u32 => None,
            },
            3u32 => phf_map!{
                2u32 => Some("2to3"),
                0u32 => Some("0to3"),
            },
        };

        let (target, migrations) = resolve(&graph, 0);
        assert_eq!(target, 3);
        assert_eq!(migrations, Some(vec!["0to3"].into_boxed_slice()));
    }
    #[test]
    fn resolve_short_path_2() {
        let graph: Graph = phf_map! {
            1u32 => phf_map!{
                0u32 => Some("0to1"),
            },
            2u32 => phf_map!{
                1u32 => None,
            },
            3u32 => phf_map!{
                2u32 => Some("2to3"),
                0u32 => Some("0to3"),
            },
            4u32 => phf_map! {
                2u32 => Some("2to4"),
            },
        };

        let (target, migrations) = resolve(&graph, 0);
        assert_eq!(target, 4);
        assert_eq!(migrations, Some(vec!["0to1", "2to4"].into_boxed_slice()));
    }
    #[test]
    fn resolve_backwards() {
        let graph: Graph = phf_map! {
            100u32 => phf_map! {},
        };

        let (target, migrations) = resolve(&graph, 1000);
        assert_eq!(target, 100);
        assert_eq!(migrations, None);
    }
}
