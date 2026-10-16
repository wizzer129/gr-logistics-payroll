// `include_dir!` doesn't tell cargo to rebuild when an embedded file is added or edited.
fn main() {
	println!("cargo:rerun-if-changed=migrations");
	println!("cargo:rerun-if-changed=templates");
	println!("cargo:rerun-if-changed=locales");
}
