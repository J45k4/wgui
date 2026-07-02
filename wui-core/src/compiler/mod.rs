pub mod codegen;
pub mod ir;
pub mod lower;
pub mod registry;
pub mod validate;

use crate::ast::Node;
use crate::compiler::ir::ActionDef;
use crate::diagnostic::Diagnostic;
use std::collections::HashMap;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub struct GeneratedModule {
	pub code: String,
	pub actions: Vec<ActionDef>,
	pub routes: Vec<(String, String)>,
	pub source_files: Vec<std::path::PathBuf>,
	pub controller_stub: Option<String>,
}

pub fn compile(source: &str, module_name: &str) -> Result<GeneratedModule, Vec<Diagnostic>> {
	compile_with_dir(source, module_name, None)
}

pub fn compile_with_dir(
	source: &str,
	module_name: &str,
	base_dir: Option<&Path>,
) -> Result<GeneratedModule, Vec<Diagnostic>> {
	let resolved = crate::imports::resolve(source, module_name, base_dir)?;
	compile_nodes_with_components(
		&resolved.nodes,
		&resolved.components,
		resolved.source_files,
		module_name,
	)
}

pub fn compile_with_loader<F>(
	source: &str,
	module_name: &str,
	base_dir: Option<&Path>,
	loader: F,
) -> Result<GeneratedModule, Vec<Diagnostic>>
where
	F: FnMut(&Path) -> io::Result<String>,
{
	let resolved = crate::imports::resolve_with_loader(source, module_name, base_dir, loader)?;
	compile_nodes_with_components(
		&resolved.nodes,
		&resolved.components,
		resolved.source_files,
		module_name,
	)
}

fn compile_nodes_with_components(
	nodes: &[Node],
	components: &HashMap<String, Vec<Node>>,
	source_files: Vec<std::path::PathBuf>,
	module_name: &str,
) -> Result<GeneratedModule, Vec<Diagnostic>> {
	let mut diags = Vec::new();
	let validated = validate::validate(nodes, components, &mut diags);
	let Some(validated) = validated else {
		return Err(diags);
	};
	let lowered = lower::lower(&validated, module_name, &mut diags);
	if !diags.is_empty() {
		return Err(diags);
	}
	let code = codegen::generate(&lowered);
	let controller_stub = codegen::generate_controller_stub(&lowered, module_name);
	let actions = lowered.actions.clone();
	let routes = lowered
		.pages
		.iter()
		.filter_map(|page| page.route.clone().map(|route| (page.module.clone(), route)))
		.collect();
	Ok(GeneratedModule {
		code,
		actions,
		routes,
		source_files,
		controller_stub,
	})
}

impl GeneratedModule {
	pub fn source_files(&self) -> Vec<std::path::PathBuf> {
		self.source_files.clone()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compiles_actions_and_routes() {
		let src = r#"
<Route path="/todo" />
<VStack>
	<Button text="Add" onClick="AddTodo" />
	<TextInput value={state.new_todo_name} onTextChanged="EditNewTodo" />
</VStack>
"#;
		let generated = compile(src, "todo").expect("compile should succeed");
		assert!(generated.code.contains("pub enum Action"));
		assert!(generated.code.contains("AddTodo"));
		assert!(generated.code.contains("EditNewTodo"));
		assert!(generated.code.contains("pub fn render(state: &"));
		assert_eq!(
			generated.routes,
			vec![("todo".to_string(), "/todo".to_string())]
		);
	}

	#[test]
	fn compiles_custom_component_event_actions() {
		let src = r#"
<CustomComponent src="/trackpad" onMouseMoved="MovePeerMouse" />
"#;
		let generated = compile(src, "trackpad").expect("compile should succeed");

		assert!(generated
			.code
			.contains("MovePeerMouse { payload: wgui::serde_json::Value }"));
		assert!(generated.code.contains("wgui::ClientEvent::OnCustom"));
		assert!(generated.code.contains("ev.name == \"mouseMoved\""));
		assert!(generated.code.contains(".custom_event(\"mouseMoved\","));
	}

	#[test]
	fn reports_unknown_tag() {
		let src = "<UnknownTag />";
		let result = compile(src, "bad");
		assert!(result.is_err());
	}
}
