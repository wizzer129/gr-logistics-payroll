// `include_dir!` doesn't tell cargo to rebuild when a migration is added or edited.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
