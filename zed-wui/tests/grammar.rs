use std::path::PathBuf;
use std::process::Command;

#[test]
fn grammar_generates() {
	let Some(tree_sitter) = find_tree_sitter() else {
		eprintln!("tree-sitter not installed; skipping grammar generation test");
		return;
	};

	let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let grammar_dir = root.join("grammars/wui");
	let status = Command::new(tree_sitter)
		.current_dir(&grammar_dir)
		.arg("generate")
		.status()
		.expect("failed to run tree-sitter generate");

	assert!(status.success(), "tree-sitter generate failed");
}

fn find_tree_sitter() -> Option<String> {
	if let Ok(path) = std::env::var("TREE_SITTER") {
		return Some(path);
	}
	let status = Command::new("tree-sitter").arg("--version").status();
	match status {
		Ok(status) if status.success() => Some("tree-sitter".to_string()),
		_ => None,
	}
}
