use std::path::PathBuf;

use wgui::wui::build::{BuildConfig, generate};

fn main() {
	println!("cargo:rerun-if-changed=wui");
	let config = BuildConfig {
		input_dir: PathBuf::from("wui"),
		output_dir: PathBuf::from("src/generated"),
		components_dir: None,
		emit_modules: false,
	};
	let result = generate(&config).unwrap_or_else(|err| panic!("{}", err));
	for path in result.source_files {
		println!("cargo:rerun-if-changed={}", path.display());
	}
}
