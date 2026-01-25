use std::fs;
use std::path::Path;

fn main() {
	let input_dir = Path::new("wui/pages");
	let output_dir = Path::new("src/generated");

	println!("cargo:rerun-if-changed=wui/pages");

	if !input_dir.exists() {
		panic!("wui input directory not found: {}", input_dir.display());
	}
	if let Err(err) = fs::create_dir_all(output_dir) {
		panic!(
			"failed to create output dir {}: {}",
			output_dir.display(),
			err
		);
	}

	let mut routes = Vec::new();
	let mut modules = Vec::new();
	let entries = fs::read_dir(input_dir).expect("failed to read wui/pages");

	for entry in entries.flatten() {
		let path = entry.path();
		if path.extension().and_then(|ext| ext.to_str()) != Some("wui") {
			continue;
		}
		println!("cargo:rerun-if-changed={}", path.display());
		let module_name = path
			.file_stem()
			.and_then(|stem| stem.to_str())
			.unwrap_or("page")
			.to_string();
		let source = fs::read_to_string(&path)
			.unwrap_or_else(|err| panic!("failed to read {}: {}", path.display(), err));
		let result = wgui::wui::compiler::compile(&source, &module_name);
		let mut generated = match result {
			Ok(gen) => gen,
			Err(diags) => {
				let mut message = format!("failed to compile {}:\n", path.display());
				for diag in diags {
					message.push_str(&format!(
						" - {} at {}..{}\n",
						diag.message, diag.span.start, diag.span.end
					));
				}
				panic!("{}", message);
			}
		};
		for (module, route) in generated.routes.drain(..) {
			routes.push((module, route));
		}
		let out_path = output_dir.join(format!("{}_gen.rs", module_name));
		fs::write(&out_path, generated.code)
			.unwrap_or_else(|err| panic!("failed to write {}: {}", out_path.display(), err));
		if let Some(stub) = generated.controller_stub.as_ref() {
			let controllers_dir = Path::new("src/controllers");
			let controller_path = controllers_dir.join(format!("{}_controller.rs", module_name));
			if !controller_path.exists() {
				fs::create_dir_all(controllers_dir).unwrap_or_else(|err| {
					panic!("failed to create {}: {}", controllers_dir.display(), err);
				});
				fs::write(&controller_path, stub).unwrap_or_else(|err| {
					panic!("failed to write {}: {}", controller_path.display(), err);
				});
			}
		}
		modules.push(module_name);
	}

	write_mod_rs(output_dir, &modules);
	write_controllers_mod(output_dir, &modules);
	write_routes(output_dir, &routes);
}

fn write_mod_rs(dir: &Path, modules: &[String]) {
	let mut contents = String::new();
	for module in modules {
		contents.push_str(&format!("pub mod {}_gen;\n", module));
	}
	contents.push_str("\n#[path = \"routes.gen.rs\"]\npub mod routes;\n");
	let out_path = dir.join("mod.rs");
	fs::write(&out_path, contents)
		.unwrap_or_else(|err| panic!("failed to write {}: {}", out_path.display(), err));
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
	fs::write(&out_path, contents)
		.unwrap_or_else(|err| panic!("failed to write {}: {}", out_path.display(), err));
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
	fs::create_dir_all(&controllers_dir)
		.unwrap_or_else(|err| panic!("failed to create {}: {}", controllers_dir.display(), err));
	let mut contents = String::new();
	for module in modules {
		contents.push_str(&format!("pub mod {}_controller;\n", module));
	}
	fs::write(&mod_path, contents)
		.unwrap_or_else(|err| panic!("failed to write {}: {}", mod_path.display(), err));
}
