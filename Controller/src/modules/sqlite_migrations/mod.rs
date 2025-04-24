pub mod graph;

use anyhow::{Context, Error, bail};

pub type Version = u32;
pub type Migration = &'static str;

pub trait Resolver {
    fn resolve(
        &self,
        current: Version,
    ) -> (Version, Option<Box<[Migration]>>); // (target, migrations)
}

pub fn execute(
    resolver: &impl Resolver,
    transaction: &rusqlite::Transaction,
) -> Result<(), Error> {
    // determine current version
    let current = sqlite_version_get(transaction).context("sqlite_version_get")?;

    // obtain list of migrations
    let (target, migrations) = resolver.resolve(current);

    // prepare migrations path
    let migrations = match migrations {
        Some(migrations) => migrations,
        None => bail!("unable to find migrations path from version {current} to {target}"),
    };

    // apply migrations
    for migration in migrations {
        transaction
            .execute_batch(migration)
            .context("execute_batch")?;
    }

    // set version on database
    if current != target {
        sqlite_version_set(target, transaction).context("sqlite_version_set")?;
    }

    Ok(())
}

const PRAGMA_VERSION: &str = "user_version";

fn sqlite_version_get(transaction: &rusqlite::Transaction) -> Result<Version, Error> {
    let version = transaction
        .pragma_query_value(None, PRAGMA_VERSION, |row| row.get::<_, Version>(0))
        .context("pragma_query_value")?;
    Ok(version)
}
fn sqlite_version_set(
    version: Version,
    transaction: &rusqlite::Transaction,
) -> Result<(), Error> {
    transaction
        .pragma_update(None, PRAGMA_VERSION, version)
        .context("pragma_update")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        graph::{Graph, GraphResolver},
        *,
    };
    use phf::phf_map;
    use std::collections::HashSet;

    #[test]
    fn execute_1() {
        let graph: Graph = phf_map! {
            100u32 => phf_map! {
                0u32 => Some("CREATE TABLE t1 (a INTEGER); CREATE TABLE t2 (b TEXT);"),
            },
        };

        let mut connection = rusqlite::Connection::open_in_memory().unwrap();
        let transaction = connection.transaction().unwrap();
        execute(&GraphResolver(&graph), &transaction).unwrap();
        transaction.commit().unwrap();

        let version = connection
            .query_row_and_then("SELECT * FROM pragma_user_version", (), |row| {
                row.get::<_, u32>(0)
            })
            .unwrap();
        assert_eq!(version, 100);
    }
    #[test]
    fn execute_2() {
        let graph: Graph = phf_map! {
            3u32 => phf_map! {
                2u32 => Some("ALTER TABLE t15 RENAME TO t1; ALTER TABLE t25 RENAME TO t2;"),
            },
            2u32 => phf_map! {
                1u32 => None,
            },
            1u32 => phf_map! {
                0u32 => Some("CREATE TABLE t15 (a INTEGER); CREATE TABLE t25 (b TEXT);"),
            },
        };

        let mut connection = rusqlite::Connection::open_in_memory().unwrap();

        let transaction = connection.transaction().unwrap();
        execute(&GraphResolver(&graph), &transaction).unwrap();
        transaction.commit().unwrap();

        let version = connection
            .query_row_and_then("SELECT * FROM pragma_user_version", (), |row| {
                row.get::<_, u32>(0)
            })
            .unwrap();
        assert_eq!(version, 3);

        let table_names = connection
            .prepare("SELECT name FROM sqlite_master")
            .unwrap()
            .query_map((), |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<HashSet<_>, _>>()
            .unwrap();
        assert_eq!(
            table_names,
            maplit::hashset! {"t1".to_owned(), "t2".to_owned()}
        );
    }
}
