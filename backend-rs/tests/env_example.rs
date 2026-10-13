//! `.env.example` is the source of truth for env keys (replaces backend/scripts/
//! check-env-parity.ts for this crate): it must list exactly the keys the code reads, and
//! any local `.env.development` / `.env.production` must have all of them.

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
fn example_lists_exactly_the_keys_the_code_reads() {
    let example = keys(&root().join(".env.example"));
    let known: BTreeSet<String> = REQUIRED_KEYS
        .iter()
        .chain(OPTIONAL_KEYS)
        .map(|key| (*key).to_owned())
        .collect();

    assert_eq!(
        example, known,
        ".env.example vs config.rs REQUIRED_KEYS + OPTIONAL_KEYS"
    );
}

/// Skipped for files that don't exist (CI has neither; the server must have .env.production).
#[test]
fn local_env_files_have_every_example_key() {
    let example = keys(&root().join(".env.example"));

    for name in [".env.development", ".env.production"] {
        let path = root().join(name);
        if !path.exists() {
            continue;
        }
        let present = keys(&path);
        let missing: Vec<_> = example.difference(&present).collect();
        assert!(missing.is_empty(), "{name} is missing {missing:?}");
    }
}
