//! `.env.schema` (varlock) is the source of truth for env keys: it must list exactly the keys
//! the code reads.

use std::{collections::BTreeSet, fs, path::Path};

use gr_logistics::config::{OPTIONAL_KEYS, REQUIRED_KEYS};

fn keys(path: &Path) -> BTreeSet<String> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('=').map(|(key, _)| key.to_owned()))
        .collect()
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn schema_lists_exactly_the_keys_the_code_reads() {
    let schema = keys(&root().join(".env.schema"));
    let known: BTreeSet<String> = REQUIRED_KEYS
        .iter()
        .chain(OPTIONAL_KEYS)
        .map(|key| (*key).to_owned())
        .collect();

    assert_eq!(
        schema, known,
        ".env.schema vs config.rs REQUIRED_KEYS + OPTIONAL_KEYS"
    );
}
