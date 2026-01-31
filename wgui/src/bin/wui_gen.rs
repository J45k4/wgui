use std::env;
use std::path::PathBuf;

use wgui::wui::build::{generate, BuildConfig};

fn main() {
	let mut args = env::args().skip(1);
	let input_dir = args
		.next()
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("wui/pages"));
	let output_dir = args
		.next()
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("src/generated"));
	let config = BuildConfig {
		input_dir,
		output_dir,
		components_dir: None,
		emit_modules: true,
	};
	if let Err(err) = generate(&config) {
		eprintln!("{}", err);
		std::process::exit(1);
	}
}
