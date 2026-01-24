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
		modules.push(module_name);
	}

	write_mod_rs(output, &modules);
	write_routes(output, &routes);
}

fn write_mod_rs(dir: &Path, modules: &[String]) {
	let mut contents = String::new();
	for module in modules {
		contents.push_str(&format!("pub mod {}_gen;\n", module));
	}
	let out_path = dir.join("mod.rs");
	if let Err(err) = fs::write(&out_path, contents) {
		eprintln!("failed to write {}: {}", out_path.display(), err);
		std::process::exit(1);
	}
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
	if let Err(err) = fs::write(&out_path, contents) {
		eprintln!("failed to write {}: {}", out_path.display(), err);
		std::process::exit(1);
	}
}
