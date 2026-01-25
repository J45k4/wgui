use std::fs;
use std::path::Path;

fn main() {
	let input_dir = std::env::args()
		.nth(1)
		.unwrap_or_else(|| "wui/pages".to_string());
	let output_dir = std::env::args()
		.nth(2)
		.unwrap_or_else(|| "src/generated".to_string());
	let input = Path::new(&input_dir);
	let output = Path::new(&output_dir);

	if !input.exists() {
		eprintln!("wui input directory not found: {}", input.display());
		std::process::exit(1);
	}
	if let Err(err) = fs::create_dir_all(output) {
		eprintln!("failed to create output dir {}: {}", output.display(), err);
		std::process::exit(1);
	}

	let mut routes = Vec::new();
	let mut modules = Vec::new();
	let entries = match fs::read_dir(input) {
		Ok(entries) => entries,
		Err(err) => {
			eprintln!("failed to read {}: {}", input.display(), err);
			std::process::exit(1);
		}
	};

	for entry in entries.flatten() {
		let path = entry.path();
		if path.extension().and_then(|ext| ext.to_str()) != Some("wui") {
			continue;
		}
		let module_name = path
			.file_stem()
			.and_then(|stem| stem.to_str())
			.unwrap_or("page")
			.to_string();
		let source = match fs::read_to_string(&path) {
			Ok(src) => src,
			Err(err) => {
				eprintln!("failed to read {}: {}", path.display(), err);
				continue;
			}
		};
		let result = wgui::wui::compiler::compile(&source, &module_name);
		let mut generated = match result {
			Ok(gen) => gen,
			Err(diags) => {
				eprintln!("failed to compile {}:", path.display());
				for diag in diags {
					eprintln!(
						" - {} at {}..{}",
						diag.message, diag.span.start, diag.span.end
					);
				}
				std::process::exit(1);
			}
		};
		for (module, route) in generated.routes.drain(..) {
			routes.push((module, route));
		}
		let out_path = output.join(format!("{}_gen.rs", module_name));
		if let Err(err) = fs::write(&out_path, generated.code) {
			eprintln!("failed to write {}: {}", out_path.display(), err);
			std::process::exit(1);
		}
		if let Some(stub) = generated.controller_stub.as_ref() {
			let controllers_dir = output
				.parent()
				.unwrap_or_else(|| Path::new("src"))
				.join("controllers");
			let controller_path = controllers_dir.join(format!("{}_controller.rs", module_name));
			if !controller_path.exists() {
				if let Err(err) = fs::create_dir_all(&controllers_dir) {
					eprintln!("failed to create {}: {}", controllers_dir.display(), err);
					std::process::exit(1);
				}
				if let Err(err) = fs::write(&controller_path, stub) {
					eprintln!("failed to write {}: {}", controller_path.display(), err);
					std::process::exit(1);
				}
			}
		}
		modules.push(module_name);
	}

	write_mod_rs(output, &modules);
	write_controllers_mod(output, &modules);
	write_routes(output, &routes);
}

fn write_mod_rs(dir: &Path, modules: &[String]) {
	let mut contents = String::new();
	for module in modules {
		contents.push_str(&format!("pub mod {}_gen;\n", module));
	}
	contents.push_str("\n#[path = \"routes.gen.rs\"]\npub mod routes;\n");
	let out_path = dir.join("mod.rs");
	if let Err(err) = fs::write(&out_path, contents) {
		eprintln!("failed to write {}: {}", out_path.display(), err);
		std::process::exit(1);
	}
}

fn write_routes(dir: &Path, routes: &[(String, String)]) {
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
		contents.push_str("\tlet routes: Vec<&'static str> = ROUTES.iter().map(|r| r.route).collect();\n");
		contents.push_str(&format!(
			"\tlet make_controller = |shared| {}::new(shared);\n",
			controller_name
		));
		contents.push_str("\twgui::wui::runtime::router_with_controller(shared, make_controller, &routes)\n");
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
	if let Err(err) = fs::write(&out_path, contents) {
		eprintln!("failed to write {}: {}", out_path.display(), err);
		std::process::exit(1);
	}
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

fn write_controllers_mod(dir: &Path, modules: &[String]) {
	let controllers_dir = dir
		.parent()
		.unwrap_or_else(|| Path::new("src"))
		.join("controllers");
	let mod_path = controllers_dir.join("mod.rs");
	if mod_path.exists() {
		return;
	}
	if let Err(err) = fs::create_dir_all(&controllers_dir) {
		eprintln!("failed to create {}: {}", controllers_dir.display(), err);
		std::process::exit(1);
	}
	let mut contents = String::new();
	for module in modules {
		contents.push_str(&format!("pub mod {}_controller;\n", module));
	}
	if let Err(err) = fs::write(&mod_path, contents) {
		eprintln!("failed to write {}: {}", mod_path.display(), err);
		std::process::exit(1);
	}
}
