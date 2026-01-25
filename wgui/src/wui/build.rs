use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::wui::compiler;
use crate::wui::diagnostic::Diagnostic;

#[derive(Clone, Debug)]
pub struct BuildConfig {
	pub input_dir: PathBuf,
	pub output_dir: PathBuf,
	pub controllers_dir: Option<PathBuf>,
}

#[derive(Debug)]
pub struct BuildResult {
	pub modules: Vec<String>,
	pub routes: Vec<(String, String)>,
	pub source_files: Vec<PathBuf>,
}

#[derive(Debug)]
pub enum BuildError {
	MissingInput(PathBuf),
	Io {
		path: PathBuf,
		source: io::Error,
	},
	Compile {
		path: PathBuf,
		diagnostics: Vec<Diagnostic>,
	},
}

impl fmt::Display for BuildError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			BuildError::MissingInput(path) => {
				write!(f, "wui input directory not found: {}", path.display())
			}
			BuildError::Io { path, source } => {
				write!(f, "I/O error on {}: {}", path.display(), source)
			}
			BuildError::Compile { path, diagnostics } => {
				write!(f, "failed to compile {}:\n", path.display())?;
				for diag in diagnostics {
					write!(
						f,
						" - {} at {}..{}\n",
						diag.message, diag.span.start, diag.span.end
					)?;
				}
				Ok(())
			}
		}
	}
}

impl Error for BuildError {}

pub fn generate(config: &BuildConfig) -> Result<BuildResult, BuildError> {
	if !config.input_dir.exists() {
		return Err(BuildError::MissingInput(config.input_dir.clone()));
	}
	fs::create_dir_all(&config.output_dir).map_err(|err| BuildError::Io {
		path: config.output_dir.clone(),
		source: err,
	})?;
	let entries = fs::read_dir(&config.input_dir).map_err(|err| BuildError::Io {
		path: config.input_dir.clone(),
		source: err,
	})?;
	let controllers_dir = config
		.controllers_dir
		.clone()
		.unwrap_or_else(|| default_controllers_dir(&config.output_dir));
	let mut modules = Vec::new();
	let mut routes = Vec::new();
	let mut source_files = Vec::new();
	for entry_result in entries {
		let entry = entry_result.map_err(|err| BuildError::Io {
			path: config.input_dir.clone(),
			source: err,
		})?;
		let path = entry.path();
		if path.extension().and_then(|ext| ext.to_str()) != Some("wui") {
			continue;
		}
		source_files.push(path.clone());
		let module_name = path
			.file_stem()
			.and_then(|stem| stem.to_str())
			.unwrap_or("page")
			.to_string();
		let source = fs::read_to_string(&path).map_err(|err| BuildError::Io {
			path: path.clone(),
			source: err,
		})?;
		let generated =
			compiler::compile(&source, &module_name).map_err(|diags| BuildError::Compile {
				path: path.clone(),
				diagnostics: diags,
			})?;
		for (module, route) in generated.routes {
			routes.push((module, route));
		}
		let out_path = config.output_dir.join(format!("{}_gen.rs", module_name));
		fs::write(&out_path, generated.code).map_err(|err| BuildError::Io {
			path: out_path.clone(),
			source: err,
		})?;
		if let Some(stub) = generated.controller_stub {
			let controller_path = controllers_dir.join(format!("{}_controller.rs", module_name));
			if !controller_path.exists() {
				fs::create_dir_all(&controllers_dir).map_err(|err| BuildError::Io {
					path: controllers_dir.clone(),
					source: err,
				})?;
				fs::write(&controller_path, stub).map_err(|err| BuildError::Io {
					path: controller_path.clone(),
					source: err,
				})?;
			}
		}
		modules.push(module_name);
	}
	write_mod_rs(&config.output_dir, &modules)?;
	write_routes(&config.output_dir, &routes)?;
	write_controllers_mod(&controllers_dir, &modules)?;
	Ok(BuildResult {
		modules,
		routes,
		source_files,
	})
}

fn write_mod_rs(dir: &Path, modules: &[String]) -> Result<(), BuildError> {
	let mut contents = String::new();
	for module in modules {
		contents.push_str(&format!("pub mod {}_gen;\n", module));
	}
	contents.push_str("\n#[path = \"routes.gen.rs\"]\npub mod routes;\n");
	let out_path = dir.join("mod.rs");
	fs::write(&out_path, contents).map_err(|err| BuildError::Io {
		path: out_path.clone(),
		source: err,
	})
}

fn write_routes(dir: &Path, routes: &[(String, String)]) -> Result<(), BuildError> {
	let mut contents = String::new();
	if let Some((module, _)) = routes.first() {
		let controller_name = format!("{}Controller", to_pascal_case(module));
		contents.push_str("#[cfg(feature = \"axum\")]\n");
		contents.push_str("use std::sync::{Arc, Mutex};\n");
		contents.push_str("#[cfg(feature = \"axum\")]\n");
		contents.push_str("use axum::Router;\n");
		contents.push_str("#[cfg(feature = \"axum\")]\n");
		contents.push_str(&format!(
			"use crate::controllers::{}_controller::{};\n",
			module, controller_name
		));
		contents.push_str("use crate::context::SharedContext;\n\n");
		contents.push_str("#[cfg(feature = \"axum\")]\n");
		contents.push_str("pub fn router(shared: Arc<Mutex<SharedContext>>) -> Router {\n");
		contents.push_str(
			"\tlet routes: Vec<&'static str> = ROUTES.iter().map(|r| r.route).collect();\n",
		);
		contents.push_str(&format!(
			"\tlet make_controller = |shared| {}::new(shared);\n",
			controller_name
		));
		contents.push_str(
			"\twgui::wui::runtime::router_with_controller(shared, make_controller, &routes)\n",
		);
		contents.push_str("}\n\n");
	}
	contents.push_str(
		"pub struct RouteDef {\n\tpub module: &'static str,\n\tpub route: &'static str,\n}\n\n",
	);
	contents.push_str("pub const ROUTES: &[RouteDef] = &[\n");
	for (module, route) in routes {
		contents.push_str(&format!(
			"\tRouteDef {{ module: \"{}\", route: \"{}\" }},\n",
			module, route
		));
	}
	contents.push_str("];\n");
	let out_path = dir.join("routes.gen.rs");
	fs::write(&out_path, contents).map_err(|err| BuildError::Io {
		path: out_path.clone(),
		source: err,
	})
}

fn write_controllers_mod(dir: &Path, modules: &[String]) -> Result<(), BuildError> {
	let mod_path = dir.join("mod.rs");
	if mod_path.exists() {
		return Ok(());
	}
	fs::create_dir_all(dir).map_err(|err| BuildError::Io {
		path: dir.to_path_buf(),
		source: err,
	})?;
	let mut contents = String::new();
	for module in modules {
		contents.push_str(&format!("pub mod {}_controller;\n", module));
	}
	fs::write(&mod_path, contents).map_err(|err| BuildError::Io {
		path: mod_path.clone(),
		source: err,
	})
}

fn to_pascal_case(input: &str) -> String {
	let mut out = String::new();
	let mut upper_next = true;
	for ch in input.chars() {
		if ch.is_ascii_alphanumeric() {
			if upper_next {
				out.push(ch.to_ascii_uppercase());
				upper_next = false;
			} else {
				out.push(ch);
			}
		} else {
			upper_next = true;
		}
	}
	if out.is_empty() {
		"Controller".to_string()
	} else {
		out
	}
}

fn default_controllers_dir(output_dir: &Path) -> PathBuf {
	output_dir
		.parent()
		.unwrap_or_else(|| Path::new("src"))
		.join("controllers")
}
