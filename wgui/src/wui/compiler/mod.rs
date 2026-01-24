pub mod codegen;
pub mod ir;
pub mod lower;
pub mod registry;
pub mod validate;

use crate::wui::diagnostic::Diagnostic;

#[derive(Debug)]
pub struct GeneratedModule {
	pub code: String,
	pub routes: Vec<(String, String)>,
}

pub fn compile(source: &str, module_name: &str) -> Result<GeneratedModule, Vec<Diagnostic>> {
	let parsed = crate::wui::parser::Parser::new(source).parse();
	let mut diags = parsed.diagnostics;
	let validated = validate::validate(&parsed.nodes, &mut diags);
	let Some(validated) = validated else {
		return Err(diags);
	};
	let lowered = lower::lower(&validated, module_name, &mut diags);
	if !diags.is_empty() {
		return Err(diags);
	}
	let code = codegen::generate(&lowered);
	let routes = lowered
		.pages
		.iter()
		.filter_map(|page| page.route.clone().map(|route| (page.module.clone(), route)))
		.collect();
	Ok(GeneratedModule { code, routes })
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compiles_actions_and_routes() {
		let src = r#"
<Page route="/todo" state="TodoState" />
<VStack>
	<Button text="Add" onClick="AddTodo" />
	<TextInput value={state.new_todo_name} onTextChanged="EditNewTodo" />
</VStack>
"#;
		let generated = compile(src, "todo").expect("compile should succeed");
		assert!(generated.code.contains("pub enum Action"));
		assert!(generated.code.contains("AddTodo"));
		assert!(generated.code.contains("EditNewTodo"));
		assert!(generated.code.contains("pub fn render(state: &TodoState)"));
		assert_eq!(
			generated.routes,
			vec![("todo".to_string(), "/todo".to_string())]
		);
	}

	#[test]
	fn reports_unknown_tag() {
		let src = "<UnknownTag />";
		let result = compile(src, "bad");
		assert!(result.is_err());
	}
}
