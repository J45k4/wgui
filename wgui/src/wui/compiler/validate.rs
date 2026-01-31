use crate::wui::ast::{AttrValue, Expr, Literal, Node};
use crate::wui::compiler::registry::{is_structural, schema_for, PropKind, ValueType};
use crate::wui::diagnostic::Diagnostic;

#[derive(Debug, Clone)]
pub struct ValidatedDocument {
	pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExprType {
	String,
	Number,
	Bool,
	Null,
	Unknown,
}

pub fn validate(nodes: &[Node], diags: &mut Vec<Diagnostic>) -> Option<ValidatedDocument> {
	for node in nodes {
		validate_node(node, diags);
	}
	if diags.is_empty() {
		Some(ValidatedDocument {
			nodes: nodes.to_vec(),
		})
	} else {
		None
	}
}

fn validate_node(node: &Node, diags: &mut Vec<Diagnostic>) {
	match node {
		Node::Element(el) => {
			if is_structural(&el.name) {
				validate_structural(el, diags);
			} else {
				validate_widget(el, diags);
			}
			for child in &el.children {
				validate_node(child, diags);
			}
		}
		Node::Text(_, _) | Node::Expr(_) => {}
	}
}

fn validate_structural(el: &crate::wui::ast::Element, diags: &mut Vec<Diagnostic>) {
	match el.name.as_str() {
		"For" => {
			require_attr(el, "each", diags);
			allow_only(el, &["each", "itemAs", "indexAs", "key"], diags);
		}
		"If" => {
			require_attr(el, "test", diags);
			allow_only(el, &["test"], diags);
		}
		"Else" => {
			if !el.attrs.is_empty() {
				diags.push(Diagnostic::new("Else does not take attributes", el.span));
			}
		}
		"Scope" => {
			require_attr(el, "name", diags);
			allow_only(el, &["name"], diags);
		}
		"Page" => {
			diags.push(Diagnostic::new(
				"Page is deprecated; use Route instead",
				el.span,
			));
			allow_only(el, &["route", "title", "state"], diags);
		}
		"Route" => {
			let has_path = el.attrs.iter().any(|attr| attr.name == "path");
			let has_route = el.attrs.iter().any(|attr| attr.name == "route");
			if !has_path && !has_route {
				diags.push(Diagnostic::new(
					"Route requires a path attribute",
					el.span,
				));
			}
			if has_path {
				require_string_attr(el, "path", diags);
			}
			if has_route {
				require_string_attr(el, "route", diags);
			}
			allow_only(el, &["path", "route", "title", "state"], diags);
		}
		"Switch" => {
			if !el.attrs.is_empty() {
				diags.push(Diagnostic::new(
					"Switch does not take attributes",
					el.span,
				));
			}
			for child in &el.children {
				match child {
					Node::Element(child_el) if child_el.name == "Case" => {}
					_ => {
						diags.push(Diagnostic::new(
							"Switch only allows Case children",
							el.span,
						));
					}
				}
			}
		}
		"Case" => {
			let has_path = el.attrs.iter().any(|attr| attr.name == "path");
			let has_route = el.attrs.iter().any(|attr| attr.name == "route");
			if !has_path && !has_route {
				diags.push(Diagnostic::new(
					"Case requires a path attribute",
					el.span,
				));
			}
			if has_path {
				require_string_attr(el, "path", diags);
			}
			if has_route {
				require_string_attr(el, "route", diags);
			}
			allow_only(el, &["path", "route", "title", "state"], diags);
		}
		"Import" => {
			require_attr(el, "src", diags);
			require_string_attr(el, "src", diags);
			allow_only(el, &["src"], diags);
			if !el.children.is_empty() {
				diags.push(Diagnostic::new(
					"Import does not take children",
					el.span,
				));
			}
		}
		_ => {}
	}
}

fn validate_widget(el: &crate::wui::ast::Element, diags: &mut Vec<Diagnostic>) {
	let Some(schema) = schema_for(&el.name) else {
		diags.push(Diagnostic::new("unknown tag", el.span));
		return;
	};
	let mut event_count = 0;
	for attr in &el.attrs {
		let Some(prop) = schema.props.iter().find(|p| p.name == attr.name) else {
			diags.push(Diagnostic::new(
				format!("unknown prop {}", attr.name),
				attr.span,
			));
			continue;
		};
		match prop.kind {
			PropKind::Event(_) => {
				event_count += 1;
				if !matches!(attr.value, AttrValue::String(_, _)) {
					diags.push(Diagnostic::new(
						"event handlers must be string literals",
						attr.span,
					));
				}
			}
			PropKind::Bind(value_type) | PropKind::Value(value_type) => {
				if !attr_value_matches(&attr.value, value_type) {
					diags.push(Diagnostic::new(
						format!("invalid value for {}", attr.name),
						attr.span,
					));
				}
			}
		}
	}
	if event_count > 1 {
		diags.push(Diagnostic::new(
			"only one event handler per element is supported",
			el.span,
		));
	}
	check_bind_conflicts(el, diags);
}

fn check_bind_conflicts(el: &crate::wui::ast::Element, diags: &mut Vec<Diagnostic>) {
	let mut has_value = false;
	let mut has_bind = false;
	for attr in &el.attrs {
		if attr.name == "value" {
			has_value = true;
		}
		if attr.name.starts_with("bind:") {
			has_bind = true;
		}
	}
	if has_value && has_bind {
		diags.push(Diagnostic::new(
			"cannot use value with bind:* on same element",
			el.span,
		));
	}
}

fn require_attr(el: &crate::wui::ast::Element, name: &str, diags: &mut Vec<Diagnostic>) {
	if !el.attrs.iter().any(|attr| attr.name == name) {
		diags.push(Diagnostic::new(
			format!("missing required attribute {}", name),
			el.span,
		));
	}
}

fn require_string_attr(el: &crate::wui::ast::Element, name: &str, diags: &mut Vec<Diagnostic>) {
	let Some(attr) = el.attrs.iter().find(|attr| attr.name == name) else {
		return;
	};
	if !matches!(attr.value, AttrValue::String(_, _)) {
		diags.push(Diagnostic::new(
			format!("{} must be a string literal", name),
			attr.span,
		));
	}
}

fn allow_only(el: &crate::wui::ast::Element, allowed: &[&str], diags: &mut Vec<Diagnostic>) {
	for attr in &el.attrs {
		if !allowed.contains(&attr.name.as_str()) {
			diags.push(Diagnostic::new(
				format!("unknown attribute {}", attr.name),
				attr.span,
			));
		}
	}
}

fn attr_value_matches(value: &AttrValue, expected: ValueType) -> bool {
	match expected {
		ValueType::String => matches!(attr_value_type(value), ExprType::String | ExprType::Unknown),
		ValueType::Number => matches!(attr_value_type(value), ExprType::Number | ExprType::Unknown),
		ValueType::Bool => matches!(attr_value_type(value), ExprType::Bool | ExprType::Unknown),
	}
}

fn attr_value_type(value: &AttrValue) -> ExprType {
	match value {
		AttrValue::String(_, _) => ExprType::String,
		AttrValue::Number(_, _) => ExprType::Number,
		AttrValue::Bool(_, _) => ExprType::Bool,
		AttrValue::Null(_) => ExprType::Null,
		AttrValue::Expr(expr) => expr_type(expr),
	}
}

fn expr_type(expr: &Expr) -> ExprType {
	match expr {
		Expr::Literal(lit, _) => match lit {
			Literal::String(_) => ExprType::String,
			Literal::Number(_) => ExprType::Number,
			Literal::Bool(_) => ExprType::Bool,
			Literal::Null => ExprType::Null,
		},
		_ => ExprType::Unknown,
	}
}
