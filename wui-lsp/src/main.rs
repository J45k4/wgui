use anyhow::Result;
use ropey::Rope;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::info;

use wgui::wui::ast::{AttrValue, Element, Node};
use wgui::wui::compiler::registry::{schema_for, PropKind, ValueType};
use wgui::wui::diagnostic::{Diagnostic as WuiDiagnostic, Span};

#[derive(Debug, Clone)]
struct Document {
	text: Rope,
	version: i32,
}

#[derive(Default)]
struct AppState {
	documents: HashMap<Url, Document>,
}

struct Backend {
	client: Client,
	state: Arc<RwLock<AppState>>,
}

#[derive(Debug, Clone)]
struct ActionRef {
	name: String,
	span: Span,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
	async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
		let capabilities = ServerCapabilities {
			text_document_sync: Some(TextDocumentSyncCapability::Kind(
				TextDocumentSyncKind::FULL,
			)),
			diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
				DiagnosticOptions {
					identifier: Some("wui".to_string()),
					inter_file_dependencies: false,
					workspace_diagnostics: false,
					work_done_progress_options: Default::default(),
				},
			)),
			completion_provider: Some(CompletionOptions {
				resolve_provider: Some(false),
				trigger_characters: Some(vec![
					"<".to_string(),
					" ".to_string(),
					"\"".to_string(),
					"{".to_string(),
					":".to_string(),
					"/".to_string(),
				]),
				..Default::default()
			}),
			hover_provider: Some(HoverProviderCapability::Simple(true)),
			definition_provider: Some(OneOf::Left(true)),
			rename_provider: Some(OneOf::Left(true)),
			..Default::default()
		};

		let _ = params;
		Ok(InitializeResult {
			capabilities,
			server_info: Some(ServerInfo {
				name: "wui-lsp".to_string(),
				version: Some("0.1.0".to_string()),
			}),
		})
	}

	async fn initialized(&self, _: InitializedParams) {
		info!("wui-lsp initialized");
	}

	async fn shutdown(&self) -> LspResult<()> {
		Ok(())
	}

	async fn did_open(&self, params: DidOpenTextDocumentParams) {
		let doc = params.text_document;
		let mut state = self.state.write().await;
		state.documents.insert(
			doc.uri.clone(),
			Document {
				text: Rope::from_str(&doc.text),
				version: doc.version,
			},
		);
		drop(state);

		self.publish_diagnostics(&doc.uri).await;
	}

	async fn did_change(&self, params: DidChangeTextDocumentParams) {
		let mut state = self.state.write().await;
		if let Some(doc) = state.documents.get_mut(&params.text_document.uri) {
			if let Some(change) = params.content_changes.last() {
				doc.text = Rope::from_str(&change.text);
				doc.version = params.text_document.version;
			}
		}
		drop(state);

		self.publish_diagnostics(&params.text_document.uri).await;
	}

	async fn did_close(&self, params: DidCloseTextDocumentParams) {
		let mut state = self.state.write().await;
		state.documents.remove(&params.text_document.uri);
		drop(state);

		self.client
			.publish_diagnostics(params.text_document.uri, Vec::new(), None)
			.await;
	}

	async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
		let uri = params.text_document_position.text_document.uri;
		let position = params.text_document_position.position;
		let text = self.get_text(&uri).await?;
		let offset = position_to_offset(&text, position);
		let actions = collect_actions(&text);

		let items = completion_items(&text, offset, &actions);
		Ok(Some(CompletionResponse::Array(items)))
	}

	async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
		let position = params.text_document_position_params.position;
		let uri = params.text_document_position_params.text_document.uri;
		let text = self.get_text(&uri).await?;
		let offset = position_to_offset(&text, position);
		let hover = hover_info(&text, offset);
		Ok(hover)
	}

	async fn goto_definition(
		&self,
		params: GotoDefinitionParams,
	) -> LspResult<Option<GotoDefinitionResponse>> {
		let uri = params.text_document_position_params.text_document.uri;
		let position = params.text_document_position_params.position;
		let text = self.get_text(&uri).await?;
		let offset = position_to_offset(&text, position);
		let actions = collect_actions(&text);
		let Some(name) = action_at_offset(&actions, offset) else {
			return Ok(None);
		};
		let Some(target) = actions.iter().find(|a| a.name == name) else {
			return Ok(None);
		};
		let range = span_to_range(&text, target.span.start, target.span.end);
		Ok(Some(GotoDefinitionResponse::Scalar(Location {
			uri,
			range,
		})))
	}

	async fn rename(&self, params: RenameParams) -> LspResult<Option<WorkspaceEdit>> {
		let uri = params.text_document_position.text_document.uri;
		let position = params.text_document_position.position;
		let new_name = params.new_name;
		let text = self.get_text(&uri).await?;
		let offset = position_to_offset(&text, position);
		let actions = collect_actions(&text);
		let Some(name) = action_at_offset(&actions, offset) else {
			return Ok(None);
		};

		let edits = actions
			.iter()
			.filter(|action| action.name == name)
			.map(|action| TextEdit {
				range: span_to_range(&text, action.span.start, action.span.end),
				new_text: new_name.clone(),
			})
			.collect::<Vec<_>>();

		if edits.is_empty() {
			return Ok(None);
		}

		let mut changes = HashMap::new();
		changes.insert(uri, edits);
		Ok(Some(WorkspaceEdit {
			changes: Some(changes),
			document_changes: None,
			change_annotations: None,
		}))
	}
}

impl Backend {
	async fn publish_diagnostics(&self, uri: &Url) {
		let (text, version) = {
			let state = self.state.read().await;
			match state.documents.get(uri) {
				Some(doc) => (doc.text.to_string(), doc.version),
				None => return,
			}
		};

		let diagnostics = analyze(&text)
			.into_iter()
			.map(|diag| to_lsp_diagnostic(&text, diag))
			.collect::<Vec<_>>();

		self.client
			.publish_diagnostics(uri.clone(), diagnostics, Some(version))
			.await;
	}

	async fn get_text(&self, uri: &Url) -> LspResult<String> {
		let state = self.state.read().await;
		match state.documents.get(uri) {
			Some(doc) => Ok(doc.text.to_string()),
			None => Ok(String::new()),
		}
	}
}

fn analyze(text: &str) -> Vec<WuiDiagnostic> {
	let parsed = wgui::wui::parser::Parser::new(text).parse();
	let mut diags = parsed.diagnostics;
	let validated = wgui::wui::compiler::validate::validate(&parsed.nodes, &mut diags);
	if validated.is_none() {
		return diags;
	}
	let _ = wgui::wui::compiler::lower::lower(
		validated.as_ref().unwrap(),
		"main",
		&mut diags,
	);
	diags
}

fn to_lsp_diagnostic(text: &str, diag: WuiDiagnostic) -> Diagnostic {
	Diagnostic {
		range: span_to_range(text, diag.span.start, diag.span.end),
		severity: Some(DiagnosticSeverity::ERROR),
		source: Some("wui".to_string()),
		message: diag.message,
		..Default::default()
	}
}

fn completion_items(text: &str, offset: usize, actions: &[ActionRef]) -> Vec<CompletionItem> {
	let mut items = Vec::new();
	if is_in_expr(text, offset) {
		items.extend(expr_completions());
	}

	if let Some(tag_ctx) = tag_context(text, offset) {
		match tag_ctx {
			TagContext::TagName => {
				items.extend(tag_completions());
			}
			TagContext::AttrName { tag } => {
				items.extend(prop_completions(&tag));
			}
			TagContext::ActionValue => {
				items.extend(action_completions(actions));
			}
		}
	}

	items
}

fn hover_info(text: &str, offset: usize) -> Option<Hover> {
	let (start, end, word) = word_at(text, offset)?;
	if let Some(tag_ctx) = tag_context(text, offset) {
		match tag_ctx {
			TagContext::TagName => {
				let content = format!("<{}>", word);
				return Some(Hover {
					contents: HoverContents::Scalar(MarkedString::String(content)),
					range: Some(span_to_range(text, start, end)),
				});
			}
			TagContext::AttrName { tag } => {
				if let Some(info) = prop_hover(&tag, &word) {
					return Some(Hover {
						contents: HoverContents::Scalar(MarkedString::String(info)),
						range: Some(span_to_range(text, start, end)),
					});
				}
			}
			TagContext::ActionValue => {
				let content = format!("Action: {}", word);
				return Some(Hover {
					contents: HoverContents::Scalar(MarkedString::String(content)),
					range: Some(span_to_range(text, start, end)),
				});
			}
		}
	}
	None
}

fn prop_hover(tag: &str, prop: &str) -> Option<String> {
	if let Some(schema) = schema_for(tag) {
		for def in schema.props {
			if def.name == prop {
				return Some(format!("{}: {}", def.name, prop_type_name(&def.kind)));
			}
		}
	}
	if let Some(prop_type) = structural_prop_type(tag, prop) {
		return Some(format!("{}: {}", prop, prop_type));
	}
	None
}

fn prop_type_name(kind: &PropKind) -> &'static str {
	match kind {
		PropKind::Value(ValueType::String) => "string",
		PropKind::Value(ValueType::Number) => "number",
		PropKind::Value(ValueType::Bool) => "bool",
		PropKind::Event(_) => "action",
		PropKind::Bind(ValueType::String) => "bind:string",
		PropKind::Bind(ValueType::Number) => "bind:number",
		PropKind::Bind(ValueType::Bool) => "bind:bool",
	}
}

fn structural_prop_type(tag: &str, prop: &str) -> Option<&'static str> {
	match tag {
		"For" => match prop {
			"each" => Some("expr"),
			"itemAs" => Some("string"),
			"indexAs" => Some("string"),
			"key" => Some("expr"),
			_ => None,
		},
		"If" => match prop {
			"test" => Some("expr"),
			_ => None,
		},
		"Scope" => match prop {
			"name" => Some("string"),
			_ => None,
		},
		"Page" => match prop {
			"route" => Some("string"),
			"title" => Some("string"),
			"state" => Some("string"),
			_ => None,
		},
		_ => None,
	}
}

fn tag_completions() -> Vec<CompletionItem> {
	let mut items = Vec::new();
	for tag in all_tags() {
		items.push(CompletionItem {
			label: tag.to_string(),
			kind: Some(CompletionItemKind::CLASS),
			..Default::default()
		});
	}
	items
}

fn prop_completions(tag: &str) -> Vec<CompletionItem> {
	let mut items = Vec::new();
	if let Some(schema) = schema_for(tag) {
		for prop in schema.props {
			items.push(CompletionItem {
				label: prop.name.to_string(),
				kind: Some(CompletionItemKind::PROPERTY),
				..Default::default()
			});
		}
	}
	for prop in structural_props(tag) {
		items.push(CompletionItem {
			label: prop.to_string(),
			kind: Some(CompletionItemKind::PROPERTY),
			..Default::default()
		});
	}
	items
}

fn action_completions(actions: &[ActionRef]) -> Vec<CompletionItem> {
	let mut items = Vec::new();
	let mut seen = HashMap::new();
	for action in actions {
		if seen.insert(action.name.clone(), ()).is_some() {
			continue;
		}
		items.push(CompletionItem {
			label: action.name.clone(),
			kind: Some(CompletionItemKind::FUNCTION),
			..Default::default()
		});
	}
	items
}

fn expr_completions() -> Vec<CompletionItem> {
	let labels = [
		"state",
		"item",
		"true",
		"false",
		"null",
		"len(",
		"trim(",
		"lower(",
		"upper(",
	];
	labels
		.iter()
		.map(|label| CompletionItem {
			label: label.to_string(),
			kind: Some(CompletionItemKind::KEYWORD),
			..Default::default()
		})
		.collect()
}

fn collect_actions(text: &str) -> Vec<ActionRef> {
	let parsed = wgui::wui::parser::Parser::new(text).parse();
	let mut out = Vec::new();
	for node in &parsed.nodes {
		collect_actions_from_node(node, &mut out);
	}
	out
}

fn collect_actions_from_node(node: &Node, out: &mut Vec<ActionRef>) {
	if let Node::Element(el) = node {
		collect_actions_from_element(el, out);
		for child in &el.children {
			collect_actions_from_node(child, out);
		}
	}
}

fn collect_actions_from_element(el: &Element, out: &mut Vec<ActionRef>) {
	for attr in &el.attrs {
		if is_event_prop(&attr.name) {
			if let AttrValue::String(name, span) = &attr.value {
				out.push(ActionRef {
					name: name.clone(),
					span: *span,
				});
			}
		}
	}
}

fn action_at_offset(actions: &[ActionRef], offset: usize) -> Option<String> {
	for action in actions {
		if offset >= action.span.start && offset <= action.span.end {
			return Some(action.name.clone());
		}
	}
	None
}

fn is_event_prop(name: &str) -> bool {
	matches!(name, "onClick" | "onTextChanged" | "onSliderChange" | "onSelect")
}

fn is_ident_char(ch: char) -> bool {
	ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' || ch == '-'
}

fn word_at(text: &str, offset: usize) -> Option<(usize, usize, String)> {
	if text.is_empty() {
		return None;
	}
	let bytes = text.as_bytes();
	let mut start = offset.min(bytes.len());
	let mut end = offset.min(bytes.len());

	while start > 0 {
		let ch = text[..start].chars().last()?;
		if is_ident_char(ch) {
			start -= ch.len_utf8();
		} else {
			break;
		}
	}
	while end < bytes.len() {
		let ch = text[end..].chars().next()?;
		if is_ident_char(ch) {
			end += ch.len_utf8();
		} else {
			break;
		}
	}

	if start == end {
		return None;
	}
	Some((start, end, text[start..end].to_string()))
}

fn is_in_expr(text: &str, offset: usize) -> bool {
	let mut depth = 0i32;
	let mut in_string = false;
	for ch in text[..offset.min(text.len())].chars() {
		if ch == '"' {
			in_string = !in_string;
		}
		if in_string {
			continue;
		}
		if ch == '{' {
			depth += 1;
		} else if ch == '}' {
			depth -= 1;
		}
	}
	depth > 0
}

enum TagContext {
	TagName,
	AttrName { tag: String },
	ActionValue,
}

fn tag_context(text: &str, offset: usize) -> Option<TagContext> {
	let before = &text[..offset.min(text.len())];
	let last_lt = before.rfind('<')?;
	let last_gt = before.rfind('>');
	if let Some(gt) = last_gt {
		if gt > last_lt {
			return None;
		}
	}
	let tag_slice = &text[last_lt..text.len()];
	let offset_in_tag = offset.saturating_sub(last_lt);
	let is_closing = tag_slice.starts_with("</");
	let name_start = if is_closing { 2 } else { 1 };
	let name_end = tag_slice[name_start..]
		.find(|ch: char| ch.is_whitespace() || ch == '/' || ch == '>')
		.map(|idx| name_start + idx)
		.unwrap_or(tag_slice.len());
	if offset_in_tag <= name_end {
		return Some(TagContext::TagName);
	}
	let tag_name = tag_slice[name_start..name_end].trim().to_string();
	if tag_name.is_empty() {
		return Some(TagContext::TagName);
	}
	if is_action_value(tag_slice, offset_in_tag) {
		return Some(TagContext::ActionValue);
	}
	Some(TagContext::AttrName { tag: tag_name })
}

fn is_action_value(tag_slice: &str, offset_in_tag: usize) -> bool {
	for event in ["onClick", "onTextChanged", "onSliderChange", "onSelect"] {
		let needle = format!("{}=\"", event);
		let Some(start_idx) = tag_slice[..offset_in_tag.min(tag_slice.len())].rfind(&needle) else {
			continue;
		};
		let value_start = start_idx + needle.len();
		let rest = &tag_slice[value_start..];
		let Some(end_rel) = rest.find('"') else {
			continue;
		};
		let value_end = value_start + end_rel;
		if offset_in_tag >= value_start && offset_in_tag <= value_end {
			return true;
		}
	}
	false
}

fn structural_props(tag: &str) -> &'static [&'static str] {
	match tag {
		"For" => &["each", "itemAs", "indexAs", "key"],
		"If" => &["test"],
		"Scope" => &["name"],
		"Page" => &["route", "title", "state"],
		_ => &[],
	}
}

fn all_tags() -> &'static [&'static str] {
	&[
		"VStack",
		"HStack",
		"Text",
		"Button",
		"TextInput",
		"Checkbox",
		"Slider",
		"Image",
		"For",
		"If",
		"Else",
		"Scope",
		"Page",
	]
}

fn span_to_range(text: &str, start: usize, end: usize) -> Range {
	Range {
		start: offset_to_position(text, start),
		end: offset_to_position(text, end),
	}
}

fn offset_to_position(text: &str, offset: usize) -> Position {
	let mut line: u32 = 0;
	let mut col: u32 = 0;
	let mut idx: usize = 0;
	for ch in text.chars() {
		let next = idx + ch.len_utf8();
		if idx >= offset {
			break;
		}
		if ch == '\n' {
			line += 1;
			col = 0;
		} else {
			col += ch.len_utf16() as u32;
		}
		idx = next;
	}
	Position { line, character: col }
}

fn position_to_offset(text: &str, position: Position) -> usize {
	let mut line: u32 = 0;
	let mut col: u32 = 0;
	let mut idx: usize = 0;
	for ch in text.chars() {
		if line == position.line && col >= position.character {
			break;
		}
		if ch == '\n' {
			line += 1;
			col = 0;
			idx += ch.len_utf8();
			continue;
		}
		col += ch.len_utf16() as u32;
		idx += ch.len_utf8();
	}
	idx
}

#[tokio::main]
async fn main() -> Result<()> {
	let mut args = std::env::args().skip(1);
	let Some(flag) = args.next() else {
		eprintln!("usage: wui-lsp --stdio");
		std::process::exit(2);
	};
	if flag != "--stdio" {
		eprintln!("unknown flag: {}", flag);
		eprintln!("usage: wui-lsp --stdio");
		std::process::exit(2);
	}

	tracing_subscriber::fmt()
		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
		.with_writer(std::io::stderr)
		.init();

	let state = Arc::new(RwLock::new(AppState::default()));
	let stdin = tokio::io::stdin();
	let stdout = tokio::io::stdout();

	let (service, socket) = LspService::new(|client| Backend { client, state });
	Server::new(stdin, stdout, socket).serve(service).await;
	Ok(())
}
