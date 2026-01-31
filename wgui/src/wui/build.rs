use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::wui::compiler;
use crate::wui::diagnostic::Diagnostic;
use crate::wui::imports;

#[derive(Clone, Debug)]
pub struct BuildConfig {
	pub input_dir: PathBuf,
	pub output_dir: PathBuf,
	pub components_dir: Option<PathBuf>,
	pub emit_modules: bool,
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
	let components_dir = config
		.components_dir
		.clone()
		.unwrap_or_else(|| default_components_dir(&config.output_dir));
	let mut modules = Vec::new();
	let mut routes = Vec::new();
	let mut source_files = Vec::new();
	let mut source_files_seen = HashSet::new();
	let component_entries = fs::read_dir(&components_dir).map_err(|err| BuildError::Io {
		path: components_dir.clone(),
		source: err,
	})?;
	for entry_result in component_entries {
		let entry = entry_result.map_err(|err| BuildError::Io {
			path: components_dir.clone(),
			source: err,
		})?;
		let component_path = entry.path();
		if component_path.file_name().and_then(|name| name.to_str()) == Some("mod.rs") {
			continue;
		}
		if component_path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
			continue;
		}
		let module_name = component_path
			.file_stem()
			.and_then(|stem| stem.to_str())
			.unwrap_or("page")
			.to_string();
		let wui_path = config.input_dir.join(format!("{module_name}.wui"));
		if !wui_path.exists() {
			return Err(BuildError::MissingInput(wui_path));
		}
		if source_files_seen.insert(wui_path.clone()) {
			source_files.push(wui_path.clone());
		}
		let source = fs::read_to_string(&wui_path).map_err(|err| BuildError::Io {
			path: wui_path.clone(),
			source: err,
		})?;
		let resolved =
			imports::resolve(&source, &module_name, wui_path.parent()).map_err(|diags| {
				BuildError::Compile {
					path: wui_path.clone(),
					diagnostics: diags,
				}
			})?;
		for import_path in resolved.source_files {
			if source_files_seen.insert(import_path.clone()) {
				source_files.push(import_path);
			}
		}
		let generated = compiler::compile_nodes(&resolved.nodes, &module_name)
			.map_err(|diags| BuildError::Compile {
				path: wui_path.clone(),
				diagnostics: diags,
			})?;
		for (module, route) in generated.routes {
			routes.push((module, route));
		}
		if config.emit_modules {
			let out_path = config.output_dir.join(format!("{}_gen.rs", module_name));
			fs::write(&out_path, generated.code).map_err(|err| BuildError::Io {
				path: out_path.clone(),
				source: err,
			})?;
			if let Some(stub) = generated.controller_stub {
				let component_path = components_dir.join(format!("{}.rs", module_name));
				if !component_path.exists() {
					fs::create_dir_all(&components_dir).map_err(|err| BuildError::Io {
						path: components_dir.clone(),
						source: err,
					})?;
					fs::write(&component_path, stub).map_err(|err| BuildError::Io {
						path: component_path.clone(),
						source: err,
					})?;
				}
			}
		}
		modules.push(module_name);
	}
	if config.emit_modules {
		write_mod_rs(&config.output_dir, &modules)?;
	} else {
		write_routes_mod(&config.output_dir)?;
	}
	write_routes(&config.output_dir, &routes)?;
	if config.emit_modules {
		write_components_mod(&components_dir, &modules)?;
	}
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

fn write_routes_mod(dir: &Path) -> Result<(), BuildError> {
	let contents = "#[path = \"routes.gen.rs\"]\npub mod routes;\n";
	let out_path = dir.join("mod.rs");
	fs::write(&out_path, contents).map_err(|err| BuildError::Io {
		path: out_path.clone(),
		source: err,
	})
}

fn write_routes(dir: &Path, routes: &[(String, String)]) -> Result<(), BuildError> {
	let mut contents = String::new();
	if let Some((module, _)) = routes.first() {
		let component_name = format!("{}", to_pascal_case(module));
		contents.push_str("#[cfg(feature = \"axum\")]\n");
		contents.push_str("use std::sync::Arc;\n");
		contents.push_str("#[cfg(feature = \"axum\")]\n");
		contents.push_str("use axum::Router;\n");
		contents.push_str("#[cfg(feature = \"axum\")]\n");
		contents.push_str(&format!(
			"use crate::components::{}::{};\n",
			module, component_name
		));
		contents.push_str("use wgui::wui::runtime::Ctx;\n");
		contents.push_str("use crate::context::SharedContext;\n\n");
		contents.push_str("#[cfg(feature = \"axum\")]\n");
		contents.push_str("pub fn router(ctx: Arc<Ctx<SharedContext>>) -> Router {\n");
		contents.push_str(
			"\tlet routes: Vec<&'static str> = ROUTES.iter().map(|r| r.route).collect();\n",
		);
		contents.push_str(&format!(
			"\twgui::wui::runtime::router_with_component::<{}>(ctx, &routes)\n",
			component_name
		));
		contents.push_str("}\n\n");
	}
	contents.push_str(
		"pub struct RouteDef {\n\tpub module: &'static str,\n\tpub route: &'static str,\n}\n\n",
	);
	contents.push_str("pub const ROUTES: &[RouteDef] = &[\n");
	for (module, route) in routes {
		let route = normalize_route(route);
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

fn write_components_mod(dir: &Path, modules: &[String]) -> Result<(), BuildError> {
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
		contents.push_str(&format!("pub mod {};\n", module));
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

fn normalize_route(route: &str) -> String {
	if route.ends_with("/*") {
		let base = route.trim_end_matches("/*");
		if base.is_empty() {
			"/{*wildcard}".to_string()
		} else {
			format!("{base}/{{*wildcard}}")
		}
	} else {
		route.to_string()
	}
}

fn default_components_dir(output_dir: &Path) -> PathBuf {
	output_dir
		.parent()
		.unwrap_or_else(|| Path::new("src"))
		.join("components")
}
