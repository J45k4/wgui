use std::path::PathBuf;

use wgui::wui::build::{generate, BuildConfig};

fn main() {
	println!("cargo:rerun-if-changed=wui/pages");
	let config = BuildConfig {
		input_dir: PathBuf::from("wui/pages"),
		output_dir: PathBuf::from("src/generated"),
		controllers_dir: None,
		emit_modules: false,
	};
	let result = generate(&config).unwrap_or_else(|err| panic!("{}", err));
	for path in result.source_files {
		println!("cargo:rerun-if-changed={}", path.display());
	}
}
