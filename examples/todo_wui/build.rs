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
		panic!("failed to create output dir {}: {}", output_dir.display(), err);
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
		modules.push(module_name);
	}

	write_mod_rs(output_dir, &modules);
	write_routes(output_dir, &routes);
}

fn write_mod_rs(dir: &Path, modules: &[String]) {
	let mut contents = String::new();
	for module in modules {
		contents.push_str(&format!("pub mod {}_gen;\n", module));
	}
	let out_path = dir.join("mod.rs");
	fs::write(&out_path, contents)
		.unwrap_or_else(|err| panic!("failed to write {}: {}", out_path.display(), err));
}

fn write_routes(dir: &Path, routes: &[(String, String)]) {
	let mut contents = String::new();
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
