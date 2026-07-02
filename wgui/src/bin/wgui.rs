use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::{Args, Parser, Subcommand, ValueEnum};
use futures_util::{SinkExt, StreamExt};
#[cfg(feature = "sqlite")]
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use wgui::wui::compiler::ir::{ActionPayload, EventKind};
use wgui::{schema_diff::diff_schemas, wdb};
use wgui::{ClientAction, Item, ItemPayload, PropKey, SetProp, Value};

#[cfg(feature = "sqlite")]
use wgui::{schema_diff_sql_from_schema_file, write_schema_migration_from_schema_file};

#[derive(Parser, Debug)]
#[command(name = "wgui")]
#[command(about = "WGUI development utilities")]
struct Cli {
	#[command(subcommand)]
	command: TopCommand,
}

#[derive(Subcommand, Debug)]
enum TopCommand {
	Migrations {
		#[command(subcommand)]
		command: MigrationsCommand,
	},
	Migrate {
		#[command(subcommand)]
		command: MigrateCommand,
	},
	Controllers {
		#[command(subcommand)]
		command: ControllersCommand,
	},
	Session {
		#[command(subcommand)]
		command: SessionCommand,
	},
	Generate(GenerateArgs),
}

#[derive(Subcommand, Debug)]
enum MigrationsCommand {
	New(NewArgs),
	Diff(DiffArgs),
	Create(CreateArgs),
	Compare(CompareArgs),
}

#[derive(Subcommand, Debug)]
enum MigrateCommand {
	Dev(MigrateDevArgs),
}

#[derive(Subcommand, Debug)]
enum ControllersCommand {
	List(ControllersListArgs),
	Call(ControllersCallArgs),
}

#[derive(Subcommand, Debug)]
enum SessionCommand {
	Start(SessionStartArgs),
	Call(SessionCallArgs),
	Inspect(SessionInspectArgs),
	Stop(SessionStopArgs),
	List,
	#[command(hide = true)]
	Daemon(SessionDaemonArgs),
}

#[derive(Args, Debug)]
struct NewArgs {
	name: String,
	#[arg(long, default_value = "migrations")]
	dir: PathBuf,
}

#[derive(Args, Debug)]
struct DiffArgs {
	#[arg(long)]
	schema: Option<PathBuf>,
	#[arg(long)]
	db: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct CreateArgs {
	name: String,
	#[arg(long)]
	schema: Option<PathBuf>,
	#[arg(long)]
	db: Option<PathBuf>,
	#[arg(long)]
	dir: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct CompareArgs {
	#[arg(long)]
	from: PathBuf,
	#[arg(long)]
	to: PathBuf,
}

#[derive(Args, Debug)]
struct MigrateDevArgs {
	#[arg(long)]
	name: String,
	#[arg(long)]
	schema: Option<PathBuf>,
	#[arg(long)]
	migrations_dir: Option<PathBuf>,
	#[arg(long)]
	env_file: Option<PathBuf>,
	#[arg(default_value = ".")]
	project_dir: PathBuf,
}

#[derive(Args, Debug)]
struct GenerateArgs {
	#[arg(default_value = ".")]
	project_dir: PathBuf,
	#[arg(long)]
	schema: Option<PathBuf>,
	#[arg(long)]
	out: Option<PathBuf>,
	#[arg(long)]
	db_name: Option<String>,
}

#[derive(Args, Debug)]
struct ControllersListArgs {
	#[arg(default_value = ".")]
	project_dir: PathBuf,
	#[arg(long)]
	json: bool,
}

#[derive(Args, Debug)]
struct ControllersCallArgs {
	action: String,
	#[arg(default_value = ".")]
	project_dir: PathBuf,
	#[arg(long, default_value = "http://127.0.0.1:12345")]
	url: String,
	#[arg(long)]
	route: Option<String>,
	#[arg(long)]
	session: Option<String>,
	#[arg(long)]
	kind: Option<ControllerEventKind>,
	#[arg(long)]
	arg: Option<u32>,
	#[arg(long)]
	value: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ControllerEventKind {
	Click,
	Press,
	Release,
	Repeat,
	TextChanged,
	SliderChange,
	Select,
}

#[derive(Args, Debug)]
struct SessionStartArgs {
	name: String,
	#[arg(default_value = ".")]
	project_dir: PathBuf,
	#[arg(long, default_value = "http://127.0.0.1:12345")]
	url: String,
	#[arg(long, default_value = "/")]
	route: String,
	#[arg(long)]
	session: Option<String>,
}

#[derive(Args, Debug)]
struct SessionCallArgs {
	name: String,
	action: String,
	#[arg(long)]
	route: Option<String>,
	#[arg(long)]
	kind: Option<ControllerEventKind>,
	#[arg(long)]
	arg: Option<u32>,
	#[arg(long)]
	value: Option<String>,
}

#[derive(Args, Debug)]
struct SessionInspectArgs {
	name: String,
	#[arg(long)]
	json: bool,
}

#[derive(Args, Debug)]
struct SessionStopArgs {
	name: String,
}

#[derive(Args, Debug)]
struct SessionDaemonArgs {
	name: String,
	project_dir: PathBuf,
	#[arg(long)]
	url: String,
	#[arg(long)]
	route: String,
	#[arg(long)]
	session: String,
}

#[derive(Debug, Default, Deserialize)]
struct WguiConfig {
	schema: Option<PathBuf>,
	db: Option<PathBuf>,
	out: Option<PathBuf>,
	db_name: Option<String>,
	migrations_dir: Option<PathBuf>,
	#[cfg(feature = "sqlite")]
	env_file: Option<PathBuf>,
}

fn main() {
	if let Err(err) = run() {
		eprintln!("wgui error: {err}");
		std::process::exit(1);
	}
}

fn run() -> Result<(), String> {
	let cli = Cli::parse();
	match cli.command {
		TopCommand::Migrations { command } => run_migrations(command),
		TopCommand::Migrate { command } => run_migrate(command),
		TopCommand::Controllers { command } => run_controllers(command),
		TopCommand::Session { command } => run_session(command),
		TopCommand::Generate(args) => run_generate(args),
	}
}

fn run_migrations(command: MigrationsCommand) -> Result<(), String> {
	match command {
		MigrationsCommand::New(args) => create_blank_migration(args),
		MigrationsCommand::Diff(args) => diff_migration(args),
		MigrationsCommand::Create(args) => create_schema_migration(args),
		MigrationsCommand::Compare(args) => compare_schemas(args),
	}
}

fn run_migrate(command: MigrateCommand) -> Result<(), String> {
	match command {
		MigrateCommand::Dev(args) => migrate_dev(args),
	}
}

fn run_controllers(command: ControllersCommand) -> Result<(), String> {
	match command {
		ControllersCommand::List(args) => list_controllers(args),
		ControllersCommand::Call(args) => call_controller(args),
	}
}

fn run_session(command: SessionCommand) -> Result<(), String> {
	match command {
		SessionCommand::Start(args) => start_session(args),
		SessionCommand::Call(args) => call_session(args),
		SessionCommand::Inspect(args) => inspect_session(args),
		SessionCommand::Stop(args) => stop_session(args),
		SessionCommand::List => list_sessions(),
		SessionCommand::Daemon(args) => run_session_daemon(args),
	}
}

fn run_generate(args: GenerateArgs) -> Result<(), String> {
	let project_dir = resolve_project_dir(&args.project_dir)?;
	let config = load_wgui_config(&project_dir)?;

	let schema_path = resolve_path_with_default(
		args.schema,
		config.schema,
		PathBuf::from("schema.wdb"),
		&project_dir,
	);
	let out_path = resolve_path_with_default(
		args.out,
		config.out,
		PathBuf::from("src/db.rs"),
		&project_dir,
	);
	let db_name = args
		.db_name
		.or(config.db_name)
		.unwrap_or_else(|| "AppDb".to_string());

	let parsed =
		wdb::parse_schema_file(&schema_path).map_err(|e| format!("failed reading schema: {e}"))?;
	let generated = generate_db_rs(&parsed, &db_name)?;

	if let Some(parent) = out_path.parent() {
		std::fs::create_dir_all(parent)
			.map_err(|e| format!("failed creating directory {}: {e}", parent.display()))?;
	}
	std::fs::write(&out_path, generated)
		.map_err(|e| format!("failed writing {}: {e}", out_path.display()))?;
	println!("generated {}", out_path.display());
	Ok(())
}

#[derive(Debug, Clone)]
struct ControllerAction {
	module: String,
	file: PathBuf,
	name: String,
	method: String,
	kind: EventKind,
	payload: ActionPayload,
	id: u32,
	routes: Vec<String>,
}

fn list_controllers(args: ControllersListArgs) -> Result<(), String> {
	let project_dir = resolve_project_dir(&args.project_dir)?;
	let actions = discover_controller_actions(&project_dir)?;
	if args.json {
		let values = actions
			.iter()
			.map(|action| {
				json!({
					"action": action.name,
					"method": action.method,
					"kind": event_kind_name(&action.kind),
					"payload": action_payload_name(&action.payload),
					"id": action.id,
					"module": action.module,
					"file": action.file.display().to_string(),
					"routes": action.routes,
				})
			})
			.collect::<Vec<_>>();
		println!(
			"{}",
			serde_json::to_string_pretty(&values)
				.map_err(|e| format!("failed serializing controller actions: {e}"))?
		);
		return Ok(());
	}

	if actions.is_empty() {
		println!("no WUI controller actions found");
		return Ok(());
	}

	for action in actions {
		let routes = if action.routes.is_empty() {
			"-".to_string()
		} else {
			action.routes.join(",")
		};
		println!(
			"{:<28} method={:<28} kind={:<12} payload={:<6} id={:<10} route={} module={}",
			action.name,
			action.method,
			event_kind_name(&action.kind),
			action_payload_name(&action.payload),
			action.id,
			routes,
			action.module
		);
	}
	Ok(())
}

fn call_controller(args: ControllersCallArgs) -> Result<(), String> {
	let project_dir = resolve_project_dir(&args.project_dir)?;
	let actions = discover_controller_actions(&project_dir)?;
	let discovered = find_controller_action(&actions, &args.action)?;
	let action_name = discovered
		.as_ref()
		.map(|action| action.name.clone())
		.unwrap_or_else(|| args.action.clone());
	let event_kind = args
		.kind
		.or_else(|| {
			discovered
				.as_ref()
				.and_then(|action| controller_event_kind_from_event(&action.kind))
		})
		.ok_or_else(|| {
			format!(
				"could not infer event kind for {}; pass --kind or add a matching WUI action",
				args.action
			)
		})?;
	let payload = discovered.as_ref().map(|action| action.payload.clone());
	let id = discovered
		.as_ref()
		.map(|action| action.id)
		.unwrap_or_else(|| action_id(&action_name));
	let route = args.route.unwrap_or_else(|| {
		discovered
			.as_ref()
			.and_then(|action| action.routes.first().cloned())
			.unwrap_or_else(|| "/".to_string())
	});
	let event = controller_event_json(id, event_kind, payload.as_ref(), args.arg, args.value)?;
	let body = websocket_messages_body(&route, Some(event))?;
	let session = args.session.unwrap_or_else(|| "wgui-cli".to_string());
	let ws_url = websocket_url(&args.url, &session);
	let rt = tokio::runtime::Runtime::new()
		.map_err(|e| format!("failed creating tokio runtime: {e}"))?;
	let ws_url_display = ws_url.clone();
	rt.block_on(async move {
		let (mut ws, _) = connect_async(&ws_url)
			.await
			.map_err(|e| format!("failed connecting to {ws_url}: {e}"))?;
		ws.send(Message::Text(body))
			.await
			.map_err(|e| format!("failed sending controller event: {e}"))?;
		tokio::time::sleep(Duration::from_millis(100)).await;
		let _ = ws.close(None).await;
		Ok::<(), String>(())
	})?;
	println!("sent {} to {}", action_name, ws_url_display);
	Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionRecord {
	name: String,
	pid: u32,
	socket: PathBuf,
	project_dir: PathBuf,
	url: String,
	route: String,
	session: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SessionDaemonRequest {
	Status,
	Inspect,
	Call {
		action: String,
		route: Option<String>,
		kind: Option<ControllerEventKind>,
		arg: Option<u32>,
		value: Option<String>,
	},
	Stop,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionDaemonResponse {
	ok: bool,
	message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct SessionSnapshot {
	root: Option<Item>,
	title: Option<String>,
	url: Option<String>,
	messages_received: u64,
	last_actions: Vec<ClientAction>,
}

fn start_session(args: SessionStartArgs) -> Result<(), String> {
	let project_dir = resolve_project_dir(&args.project_dir)?;
	let session = args
		.session
		.unwrap_or_else(|| format!("wgui-cli-{}", sanitize_session_name(&args.name)));
	let socket_path = session_socket_path(&args.name)?;
	if socket_path.exists()
		&& session_request(&args.name, &SessionDaemonRequest::Status)
			.map(|response| response.ok)
			.unwrap_or(false)
	{
		return Err(format!("session {} is already running", args.name));
	}
	let _ = std::fs::remove_file(&socket_path);
	let _ = std::fs::remove_file(session_record_path(&args.name)?);

	let exe = std::env::current_exe().map_err(|e| format!("failed locating wgui binary: {e}"))?;
	spawn_session_daemon(
		&exe,
		&args.name,
		&project_dir,
		&args.url,
		&args.route,
		&session,
	)?;

	let started = std::time::Instant::now();
	loop {
		if started.elapsed() > Duration::from_secs(5) {
			return Err(format!("session {} did not start within 5s", args.name));
		}
		if session_request(&args.name, &SessionDaemonRequest::Status)
			.map(|response| response.ok)
			.unwrap_or(false)
		{
			println!("started session {}", args.name);
			return Ok(());
		}
		std::thread::sleep(Duration::from_millis(100));
	}
}

fn spawn_session_daemon(
	exe: &std::path::Path,
	name: &str,
	project_dir: &std::path::Path,
	url: &str,
	route: &str,
	session: &str,
) -> Result<(), String> {
	let add_args = |command: &mut ProcessCommand| {
		command
			.arg("session")
			.arg("daemon")
			.arg(name)
			.arg(project_dir)
			.arg("--url")
			.arg(url)
			.arg("--route")
			.arg(route)
			.arg("--session")
			.arg(session)
			.stdin(Stdio::null())
			.stdout(Stdio::null())
			.stderr(Stdio::null());
	};

	let mut detached = ProcessCommand::new("setsid");
	detached.arg(exe);
	add_args(&mut detached);
	match detached.spawn() {
		Ok(_) => Ok(()),
		Err(detached_err) => {
			let mut direct = ProcessCommand::new(exe);
			add_args(&mut direct);
			direct.spawn().map(|_| ()).map_err(|direct_err| {
				format!(
					"failed starting session daemon: {direct_err}; setsid fallback failed first: {detached_err}"
				)
			})
		}
	}
}

fn call_session(args: SessionCallArgs) -> Result<(), String> {
	let response = session_request(
		&args.name,
		&SessionDaemonRequest::Call {
			action: args.action,
			route: args.route,
			kind: args.kind,
			arg: args.arg,
			value: args.value,
		},
	)?;
	if response.ok {
		println!("{}", response.message);
		Ok(())
	} else {
		Err(response.message)
	}
}

fn inspect_session(args: SessionInspectArgs) -> Result<(), String> {
	let response = session_request(&args.name, &SessionDaemonRequest::Inspect)?;
	if !response.ok {
		return Err(response.message);
	}
	let data = response
		.data
		.ok_or_else(|| "session did not return inspect data".to_string())?;
	if args.json {
		println!(
			"{}",
			serde_json::to_string(&data)
				.map_err(|e| format!("failed serializing inspect data: {e}"))?
		);
	} else {
		println!(
			"{}",
			serde_json::to_string_pretty(&data)
				.map_err(|e| format!("failed serializing inspect data: {e}"))?
		);
	}
	Ok(())
}

fn stop_session(args: SessionStopArgs) -> Result<(), String> {
	let response = session_request(&args.name, &SessionDaemonRequest::Stop)?;
	if response.ok {
		println!("{}", response.message);
		Ok(())
	} else {
		Err(response.message)
	}
}

fn list_sessions() -> Result<(), String> {
	let dir = session_dir()?;
	if !dir.exists() {
		println!("no sessions");
		return Ok(());
	}
	let mut records = Vec::new();
	for entry in std::fs::read_dir(&dir)
		.map_err(|e| format!("failed reading session dir {}: {e}", dir.display()))?
	{
		let entry = entry.map_err(|e| format!("failed reading session entry: {e}"))?;
		let path = entry.path();
		if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
			continue;
		}
		let raw = std::fs::read_to_string(&path)
			.map_err(|e| format!("failed reading session record {}: {e}", path.display()))?;
		let record: SessionRecord = serde_json::from_str(&raw)
			.map_err(|e| format!("failed parsing session record {}: {e}", path.display()))?;
		records.push(record);
	}
	records.sort_by(|a, b| a.name.cmp(&b.name));
	if records.is_empty() {
		println!("no sessions");
		return Ok(());
	}
	for record in records {
		let status = session_request(&record.name, &SessionDaemonRequest::Status)
			.map(|response| if response.ok { "running" } else { "unhealthy" })
			.unwrap_or("stale");
		println!(
			"{:<20} {:<9} pid={} route={} url={}",
			record.name, status, record.pid, record.route, record.url
		);
	}
	Ok(())
}

fn run_session_daemon(args: SessionDaemonArgs) -> Result<(), String> {
	let rt = tokio::runtime::Runtime::new()
		.map_err(|e| format!("failed creating tokio runtime: {e}"))?;
	rt.block_on(run_session_daemon_async(args))
}

async fn run_session_daemon_async(args: SessionDaemonArgs) -> Result<(), String> {
	let project_dir = resolve_project_dir(&args.project_dir)?;
	let socket_path = session_socket_path(&args.name)?;
	let record_path = session_record_path(&args.name)?;
	if let Some(parent) = socket_path.parent() {
		std::fs::create_dir_all(parent)
			.map_err(|e| format!("failed creating session dir {}: {e}", parent.display()))?;
	}
	let _ = std::fs::remove_file(&socket_path);

	let ws_url = websocket_url(&args.url, &args.session);
	let (ws, _) = connect_async(&ws_url)
		.await
		.map_err(|e| format!("failed connecting to {ws_url}: {e}"))?;
	let (mut ws_write, mut ws_read) = ws.split();
	let snapshot = Arc::new(Mutex::new(SessionSnapshot::default()));
	let reader_snapshot = snapshot.clone();
	tokio::spawn(async move {
		while let Some(message) = ws_read.next().await {
			let Ok(message) = message else {
				continue;
			};
			if let Message::Text(text) = message {
				if let Ok(actions) = serde_json::from_str::<Vec<ClientAction>>(&text) {
					update_session_snapshot(&reader_snapshot, actions);
				}
			}
		}
	});
	ws_write
		.send(Message::Text(websocket_messages_body(&args.route, None)?))
		.await
		.map_err(|e| format!("failed mounting session route: {e}"))?;

	let listener = UnixListener::bind(&socket_path).map_err(|e| {
		format!(
			"failed binding session socket {}: {e}",
			socket_path.display()
		)
	})?;
	let record = SessionRecord {
		name: args.name.clone(),
		pid: std::process::id(),
		socket: socket_path.clone(),
		project_dir: project_dir.clone(),
		url: args.url.clone(),
		route: args.route.clone(),
		session: args.session.clone(),
	};
	write_session_record(&record_path, &record)?;

	loop {
		let (mut stream, _) = listener
			.accept()
			.await
			.map_err(|e| format!("failed accepting session request: {e}"))?;
		let mut raw = String::new();
		stream
			.read_to_string(&mut raw)
			.await
			.map_err(|e| format!("failed reading session request: {e}"))?;
		let request: SessionDaemonRequest = match serde_json::from_str(&raw) {
			Ok(request) => request,
			Err(err) => {
				write_daemon_response(&mut stream, false, &format!("invalid request: {err}"))
					.await?;
				continue;
			}
		};

		match request {
			SessionDaemonRequest::Status => {
				write_daemon_response(&mut stream, true, "running").await?;
			}
			SessionDaemonRequest::Inspect => {
				let data = {
					let snapshot = snapshot.lock().unwrap().clone();
					serde_json::to_value(snapshot)
						.map_err(|e| format!("failed serializing session snapshot: {e}"))?
				};
				write_daemon_response_data(&mut stream, true, "ok", Some(data)).await?;
			}
			SessionDaemonRequest::Call {
				action,
				route,
				kind,
				arg,
				value,
			} => {
				let result = session_daemon_call(
					&project_dir,
					&mut ws_write,
					&args.route,
					action,
					route,
					kind,
					arg,
					value,
				)
				.await;
				match result {
					Ok(message) => write_daemon_response(&mut stream, true, &message).await?,
					Err(err) => write_daemon_response(&mut stream, false, &err).await?,
				}
			}
			SessionDaemonRequest::Stop => {
				write_daemon_response(&mut stream, true, &format!("stopped session {}", args.name))
					.await?;
				break;
			}
		}
	}

	let _ = std::fs::remove_file(&socket_path);
	let _ = std::fs::remove_file(&record_path);
	let _ = ws_write.close().await;
	Ok(())
}

async fn session_daemon_call<S>(
	project_dir: &std::path::Path,
	ws_write: &mut S,
	default_route: &str,
	action: String,
	route: Option<String>,
	kind: Option<ControllerEventKind>,
	arg: Option<u32>,
	value: Option<String>,
) -> Result<String, String>
where
	S: SinkExt<Message> + Unpin,
	<S as futures_util::Sink<Message>>::Error: std::fmt::Display,
{
	let actions = discover_controller_actions(project_dir)?;
	let discovered = find_controller_action(&actions, &action)?;
	let action_name = discovered
		.as_ref()
		.map(|action| action.name.clone())
		.unwrap_or(action);
	let event_kind = kind
		.or_else(|| {
			discovered
				.as_ref()
				.and_then(|action| controller_event_kind_from_event(&action.kind))
		})
		.ok_or_else(|| {
			format!("could not infer event kind for {action_name}; pass --kind on session call")
		})?;
	let payload = discovered.as_ref().map(|action| action.payload.clone());
	let id = discovered
		.as_ref()
		.map(|action| action.id)
		.unwrap_or_else(|| action_id(&action_name));
	let route = route.unwrap_or_else(|| default_route.to_string());
	let event = controller_event_json(id, event_kind, payload.as_ref(), arg, value)?;
	let body = websocket_messages_body(&route, Some(event))?;
	ws_write
		.send(Message::Text(body))
		.await
		.map_err(|e| format!("failed sending controller event: {e}"))?;
	Ok(format!("sent {action_name}"))
}

async fn write_daemon_response(
	stream: &mut tokio::net::UnixStream,
	ok: bool,
	message: &str,
) -> Result<(), String> {
	write_daemon_response_data(stream, ok, message, None).await
}

async fn write_daemon_response_data(
	stream: &mut tokio::net::UnixStream,
	ok: bool,
	message: &str,
	data: Option<serde_json::Value>,
) -> Result<(), String> {
	let response = SessionDaemonResponse {
		ok,
		message: message.to_string(),
		data,
	};
	let raw = serde_json::to_vec(&response)
		.map_err(|e| format!("failed serializing daemon response: {e}"))?;
	stream
		.write_all(&raw)
		.await
		.map_err(|e| format!("failed writing daemon response: {e}"))
}

fn update_session_snapshot(snapshot: &Arc<Mutex<SessionSnapshot>>, actions: Vec<ClientAction>) {
	let mut snapshot = snapshot.lock().unwrap();
	snapshot.messages_received += 1;
	snapshot.last_actions = actions.clone();
	for action in actions {
		apply_client_action(&mut snapshot, action);
	}
}

fn apply_client_action(snapshot: &mut SessionSnapshot, action: ClientAction) {
	match action {
		ClientAction::Replace(replace) => {
			if replace.path.is_empty() {
				snapshot.root = Some(replace.item);
			} else if let Some(root) = snapshot.root.as_mut() {
				if let Some(slot) = item_at_path_mut(root, &replace.path) {
					*slot = replace.item;
				}
			}
		}
		ClientAction::AddBack(add) => {
			if let Some(parent) = snapshot
				.root
				.as_mut()
				.and_then(|root| item_at_path_mut(root, &add.path))
				.and_then(item_children_mut)
			{
				parent.push(add.item);
			}
		}
		ClientAction::AddFront(add) => {
			if let Some(parent) = snapshot
				.root
				.as_mut()
				.and_then(|root| item_at_path_mut(root, &add.path))
				.and_then(item_children_mut)
			{
				parent.insert(0, add.item);
			}
		}
		ClientAction::InsertAt(insert) => {
			if let Some(parent) = snapshot
				.root
				.as_mut()
				.and_then(|root| item_at_path_mut(root, &insert.path))
				.and_then(item_children_mut)
			{
				let index = (insert.inx + 1).min(parent.len());
				parent.insert(index, insert.item);
			}
		}
		ClientAction::ReplaceAt(replace) => {
			if let Some(parent) = snapshot
				.root
				.as_mut()
				.and_then(|root| item_at_path_mut(root, &replace.path))
				.and_then(item_children_mut)
			{
				if let Some(slot) = parent.get_mut(replace.inx) {
					*slot = replace.item;
				}
			}
		}
		ClientAction::RemoveInx(remove) => {
			if let Some(parent) = snapshot
				.root
				.as_mut()
				.and_then(|root| item_at_path_mut(root, &remove.path))
				.and_then(item_children_mut)
			{
				if remove.inx < parent.len() {
					parent.remove(remove.inx);
				}
			}
		}
		ClientAction::SetProp { path, sets } => {
			if let Some(item) = snapshot
				.root
				.as_mut()
				.and_then(|root| item_at_path_mut(root, &path))
			{
				for set in sets {
					apply_set_prop(item, set);
				}
			}
		}
		ClientAction::SetTitle { title } => {
			snapshot.title = Some(title);
		}
		ClientAction::PushState(push) => {
			snapshot.url = Some(push.url);
		}
		ClientAction::Navigate(navigate) => {
			snapshot.url = Some(navigate.url);
		}
		ClientAction::ReplaceState(replace) => {
			snapshot.url = Some(replace.url);
		}
		ClientAction::SetQuery(_)
		| ClientAction::ThreePatch { .. }
		| ClientAction::WebRtcRoomState { .. }
		| ClientAction::WebRtcSignal { .. }
		| ClientAction::WebPushEnable { .. }
		| ClientAction::WebPushDisable { .. }
		| ClientAction::CustomData(_) => {}
	}
}

fn item_at_path_mut<'a>(item: &'a mut Item, path: &[usize]) -> Option<&'a mut Item> {
	if path.is_empty() {
		return Some(item);
	}
	let (first, rest) = path.split_first()?;
	let children = item_children_mut(item)?;
	let child = children.get_mut(*first)?;
	item_at_path_mut(child, rest)
}

fn item_children_mut(item: &mut Item) -> Option<&mut Vec<Item>> {
	match &mut item.payload {
		ItemPayload::Layout(layout) => Some(&mut layout.body),
		ItemPayload::Table { items }
		| ItemPayload::Thead { items }
		| ItemPayload::Tbody { items }
		| ItemPayload::Tr { items } => Some(items),
		ItemPayload::Modal { body, .. } => Some(body),
		_ => None,
	}
}

fn apply_set_prop(item: &mut Item, set: SetProp) {
	match (set.key, set.value) {
		(PropKey::ID, Value::Number(value)) => item.id = value,
		(PropKey::Border, Value::String(value)) => item.border = value,
		(PropKey::BackgroundColor, Value::String(value)) => item.background_color = value,
		(PropKey::Color, Value::String(value)) => item.color = value,
		(PropKey::Spacing, Value::Number(value)) => {
			if let ItemPayload::Layout(layout) = &mut item.payload {
				layout.spacing = value;
			}
		}
		(PropKey::FlexDirection, Value::String(value)) => {
			if let ItemPayload::Layout(layout) = &mut item.payload {
				layout.flex = if value == "row" {
					wgui::FlexDirection::Row
				} else {
					wgui::FlexDirection::Column
				};
			}
		}
		(PropKey::Grow, Value::Number(value)) => item.grow = value,
		(PropKey::Width, Value::Number(value)) => item.width = value,
		(PropKey::Height, Value::Number(value)) => item.height = value,
		(PropKey::MinWidth, Value::Number(value)) => item.min_width = value,
		(PropKey::MaxWidth, Value::Number(value)) => item.max_width = value,
		(PropKey::MinHeight, Value::Number(value)) => item.min_height = value,
		(PropKey::MaxHeight, Value::Number(value)) => item.max_height = value,
		(PropKey::Padding, Value::Number(value)) => item.padding = value as u16,
		(PropKey::Overflow, Value::String(value)) => item.overflow = value,
		(PropKey::BreakWords, Value::Number(value)) => item.break_words = value != 0,
		(PropKey::Fill, Value::Number(value)) => item.fill = value != 0,
		_ => {}
	}
}

fn session_request(
	name: &str,
	request: &SessionDaemonRequest,
) -> Result<SessionDaemonResponse, String> {
	let socket_path = session_socket_path(name)?;
	let mut stream = std::os::unix::net::UnixStream::connect(&socket_path)
		.map_err(|e| format!("failed connecting to session {}: {e}", name))?;
	let raw = serde_json::to_vec(request)
		.map_err(|e| format!("failed serializing session request: {e}"))?;
	stream
		.write_all(&raw)
		.map_err(|e| format!("failed writing session request: {e}"))?;
	stream
		.shutdown(std::net::Shutdown::Write)
		.map_err(|e| format!("failed finishing session request: {e}"))?;
	let mut response = String::new();
	stream
		.read_to_string(&mut response)
		.map_err(|e| format!("failed reading session response: {e}"))?;
	serde_json::from_str(&response)
		.map_err(|e| format!("failed parsing session response: {e}: {response}"))
}

fn write_session_record(path: &std::path::Path, record: &SessionRecord) -> Result<(), String> {
	let raw = serde_json::to_string_pretty(record)
		.map_err(|e| format!("failed serializing session record: {e}"))?;
	std::fs::write(path, raw)
		.map_err(|e| format!("failed writing session record {}: {e}", path.display()))
}

fn session_dir() -> Result<PathBuf, String> {
	let dir = std::env::temp_dir().join("wgui-sessions");
	std::fs::create_dir_all(&dir)
		.map_err(|e| format!("failed creating session dir {}: {e}", dir.display()))?;
	Ok(dir)
}

fn session_socket_path(name: &str) -> Result<PathBuf, String> {
	Ok(session_dir()?.join(format!("{}.sock", sanitize_session_name(name))))
}

fn session_record_path(name: &str) -> Result<PathBuf, String> {
	Ok(session_dir()?.join(format!("{}.json", sanitize_session_name(name))))
}

fn sanitize_session_name(name: &str) -> String {
	let mut out = String::new();
	for ch in name.chars() {
		if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
			out.push(ch);
		} else if !out.ends_with('_') {
			out.push('_');
		}
	}
	let out = out.trim_matches('_');
	if out.is_empty() {
		"default".to_string()
	} else {
		out.to_string()
	}
}

fn resolve_project_dir(project_dir: &std::path::Path) -> Result<PathBuf, String> {
	std::fs::canonicalize(project_dir).map_err(|e| {
		format!(
			"failed to resolve project dir {}: {e}",
			project_dir.display()
		)
	})
}

fn load_wgui_config(project_dir: &std::path::Path) -> Result<WguiConfig, String> {
	let path = project_dir.join("wgui.toml");
	if !path.exists() {
		return Ok(WguiConfig::default());
	}
	let raw = std::fs::read_to_string(&path)
		.map_err(|e| format!("failed reading {}: {e}", path.display()))?;
	toml::from_str(&raw).map_err(|e| format!("failed parsing {}: {e}", path.display()))
}

fn resolve_path_with_default(
	cli: Option<PathBuf>,
	config: Option<PathBuf>,
	default: PathBuf,
	base: &std::path::Path,
) -> PathBuf {
	let raw = cli.or(config).unwrap_or(default);
	if raw.is_absolute() {
		raw
	} else {
		base.join(raw)
	}
}

fn discover_controller_actions(
	project_dir: &std::path::Path,
) -> Result<Vec<ControllerAction>, String> {
	let wui_dir = project_dir.join("wui");
	if !wui_dir.exists() {
		return Ok(Vec::new());
	}

	let mut files = Vec::new();
	collect_wui_files(&wui_dir, &mut files)?;
	files.sort();

	let mut actions = Vec::new();
	for file in files {
		let module = module_name_for_wui_file(&wui_dir, &file)?;
		let source = std::fs::read_to_string(&file)
			.map_err(|e| format!("failed reading {}: {e}", file.display()))?;
		let generated = wgui::wui::compiler::compile_with_dir(&source, &module, file.parent())
			.map_err(|diags| format_wui_diagnostics(&file, &diags))?;
		let routes = generated
			.routes
			.iter()
			.map(|(_, route)| route.clone())
			.collect::<Vec<_>>();
		for action in generated.actions {
			actions.push(ControllerAction {
				module: module.clone(),
				file: file.clone(),
				method: action_method_name(&action.name),
				name: action.name,
				kind: action.kind,
				payload: action.payload,
				id: action.id,
				routes: routes.clone(),
			});
		}
	}

	actions.sort_by(|a, b| {
		a.module
			.cmp(&b.module)
			.then_with(|| a.name.cmp(&b.name))
			.then_with(|| a.id.cmp(&b.id))
	});
	actions.dedup_by(|a, b| {
		a.module == b.module
			&& a.name == b.name
			&& a.kind == b.kind
			&& a.payload == b.payload
			&& a.id == b.id
	});
	Ok(actions)
}

fn collect_wui_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
	for entry in std::fs::read_dir(dir)
		.map_err(|e| format!("failed reading directory {}: {e}", dir.display()))?
	{
		let entry = entry.map_err(|e| format!("failed reading directory entry: {e}"))?;
		let path = entry.path();
		let ty = entry
			.file_type()
			.map_err(|e| format!("failed reading file type {}: {e}", path.display()))?;
		if ty.is_dir() {
			collect_wui_files(&path, out)?;
		} else if path.extension().and_then(|ext| ext.to_str()) == Some("wui") {
			out.push(path);
		}
	}
	Ok(())
}

fn module_name_for_wui_file(
	base: &std::path::Path,
	file: &std::path::Path,
) -> Result<String, String> {
	let rel = file
		.strip_prefix(base)
		.map_err(|e| format!("failed resolving module name for {}: {e}", file.display()))?;
	let mut parts = rel
		.components()
		.filter_map(|component| match component {
			std::path::Component::Normal(value) => value.to_str().map(|value| value.to_string()),
			_ => None,
		})
		.collect::<Vec<_>>();
	if let Some(last) = parts.last_mut() {
		if let Some(stripped) = last.strip_suffix(".wui") {
			*last = stripped.to_string();
		}
	}
	Ok(parts.join("/"))
}

fn format_wui_diagnostics(
	file: &std::path::Path,
	diags: &[wgui::wui::diagnostic::Diagnostic],
) -> String {
	let details = diags
		.iter()
		.map(|diag| {
			format!(
				"{}:{}-{}: {}",
				file.display(),
				diag.span.start,
				diag.span.end,
				diag.message
			)
		})
		.collect::<Vec<_>>()
		.join("\n");
	format!("failed compiling WUI template:\n{details}")
}

fn find_controller_action(
	actions: &[ControllerAction],
	query: &str,
) -> Result<Option<ControllerAction>, String> {
	let matches = actions
		.iter()
		.filter(|action| action.name == query || action.method == query)
		.collect::<Vec<_>>();
	if matches.is_empty() {
		return Ok(None);
	}
	let first = matches[0];
	let ambiguous = matches.iter().any(|action| {
		action.name != first.name || action.kind != first.kind || action.payload != first.payload
	});
	if ambiguous {
		let names = matches
			.iter()
			.map(|action| format!("{} ({})", action.name, action.module))
			.collect::<Vec<_>>()
			.join(", ");
		return Err(format!("controller action {query} is ambiguous: {names}"));
	}
	Ok(Some(first.clone()))
}

fn controller_event_json(
	id: u32,
	kind: ControllerEventKind,
	payload: Option<&ActionPayload>,
	arg: Option<u32>,
	value: Option<String>,
) -> Result<serde_json::Value, String> {
	if let Some(payload) = payload {
		match payload {
			ActionPayload::None if arg.is_some() || value.is_some() => {
				return Err("this action does not accept --arg or --value".to_string());
			}
			ActionPayload::U32 if arg.is_none() => {
				return Err("this action requires --arg <u32>".to_string());
			}
			ActionPayload::String if value.is_none() => {
				return Err("this action requires --value <text>".to_string());
			}
			ActionPayload::I32 if value.is_none() => {
				return Err("this action requires --value <i32>".to_string());
			}
			ActionPayload::U32I32 if arg.is_none() || value.is_none() => {
				return Err("this action requires --arg <u32> and --value <i32>".to_string());
			}
			ActionPayload::Json => {
				return Err("custom events cannot be called with --kind".to_string());
			}
			_ => {}
		}
	}

	match kind {
		ControllerEventKind::Click => {
			let mut event = json!({ "type": "onClick", "id": id });
			if let Some(arg) = arg {
				event["inx"] = json!(arg);
			}
			Ok(event)
		}
		ControllerEventKind::Press => {
			let mut event = json!({ "type": "onPress", "id": id });
			if let Some(arg) = arg {
				event["inx"] = json!(arg);
			}
			Ok(event)
		}
		ControllerEventKind::Release => {
			let mut event = json!({ "type": "onRelease", "id": id });
			if let Some(arg) = arg {
				event["inx"] = json!(arg);
			}
			Ok(event)
		}
		ControllerEventKind::Repeat => {
			let mut event = json!({ "type": "onRepeat", "id": id });
			if let Some(arg) = arg {
				event["inx"] = json!(arg);
			}
			Ok(event)
		}
		ControllerEventKind::TextChanged => Ok(json!({
			"type": "onTextChanged",
			"id": id,
			"value": value.unwrap_or_default(),
		})),
		ControllerEventKind::SliderChange => {
			let raw = value.unwrap_or_else(|| "0".to_string());
			let parsed = raw
				.parse::<i32>()
				.map_err(|e| format!("--value must be an i32 for slider-change: {e}"))?;
			let mut event = json!({ "type": "onSliderChange", "id": id, "value": parsed });
			if let Some(arg) = arg {
				event["inx"] = json!(arg);
			}
			Ok(event)
		}
		ControllerEventKind::Select => Ok(json!({
			"type": "onSelect",
			"id": id,
			"value": value.unwrap_or_default(),
		})),
	}
}

fn controller_event_kind_from_event(kind: &EventKind) -> Option<ControllerEventKind> {
	match kind {
		EventKind::Click => Some(ControllerEventKind::Click),
		EventKind::Press => Some(ControllerEventKind::Press),
		EventKind::Release => Some(ControllerEventKind::Release),
		EventKind::Repeat => Some(ControllerEventKind::Repeat),
		EventKind::TextChanged => Some(ControllerEventKind::TextChanged),
		EventKind::SliderChange => Some(ControllerEventKind::SliderChange),
		EventKind::Select => Some(ControllerEventKind::Select),
		EventKind::Custom(_) => None,
	}
}

fn event_kind_name(kind: &EventKind) -> String {
	match kind {
		EventKind::Click => "click".to_string(),
		EventKind::Press => "press".to_string(),
		EventKind::Release => "release".to_string(),
		EventKind::Repeat => "repeat".to_string(),
		EventKind::TextChanged => "text-changed".to_string(),
		EventKind::SliderChange => "slider-change".to_string(),
		EventKind::Select => "select".to_string(),
		EventKind::Custom(name) => format!("custom:{name}"),
	}
}

fn action_payload_name(payload: &ActionPayload) -> &'static str {
	match payload {
		ActionPayload::None => "none",
		ActionPayload::U32 => "u32",
		ActionPayload::String => "string",
		ActionPayload::I32 => "i32",
		ActionPayload::U32I32 => "u32,i32",
		ActionPayload::Json => "json",
	}
}

fn split_route(route: &str) -> (String, HashMap<String, String>) {
	let Some((path, query)) = route.split_once('?') else {
		return (route.to_string(), HashMap::new());
	};
	let query = query
		.split('&')
		.filter_map(|entry| {
			let (key, value) = entry.split_once('=')?;
			Some((key.to_string(), value.to_string()))
		})
		.collect();
	(path.to_string(), query)
}

fn websocket_messages_body(
	route: &str,
	event: Option<serde_json::Value>,
) -> Result<String, String> {
	let (path, query) = split_route(route);
	let mut messages = vec![json!({
		"type": "pathChanged",
		"path": path,
		"query": query,
	})];
	if let Some(event) = event {
		messages.push(event);
	}
	serde_json::to_string(&messages)
		.map_err(|e| format!("failed serializing websocket messages: {e}"))
}

fn websocket_url(base: &str, session: &str) -> String {
	let trimmed = base.trim().trim_end_matches('/');
	let mut url = if trimmed.starts_with("ws://") || trimmed.starts_with("wss://") {
		trimmed.to_string()
	} else if let Some(rest) = trimmed.strip_prefix("http://") {
		format!("ws://{rest}")
	} else if let Some(rest) = trimmed.strip_prefix("https://") {
		format!("wss://{rest}")
	} else {
		format!("ws://{trimmed}")
	};
	if !url.contains("/ws") {
		url.push_str("/ws");
	}
	let sep = if url.contains('?') { '&' } else { '?' };
	format!("{url}{sep}sid={}", percent_encode_query_value(session))
}

fn percent_encode_query_value(value: &str) -> String {
	let mut out = String::new();
	for byte in value.bytes() {
		if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
			out.push(byte as char);
		} else {
			out.push_str(&format!("%{byte:02X}"));
		}
	}
	out
}

fn action_id(name: &str) -> u32 {
	let mut hash = 0x811c9dc5u32;
	for byte in name.as_bytes() {
		hash ^= *byte as u32;
		hash = hash.wrapping_mul(0x01000193);
	}
	if hash == 0 {
		1
	} else {
		hash
	}
}

fn action_method_name(name: &str) -> String {
	let mut out = String::new();
	let mut prev_underscore = false;
	for (i, ch) in name.chars().enumerate() {
		if ch.is_ascii_alphanumeric() {
			if ch.is_ascii_uppercase() {
				if i != 0 && !prev_underscore {
					out.push('_');
				}
				out.push(ch.to_ascii_lowercase());
				prev_underscore = false;
			} else {
				out.push(ch.to_ascii_lowercase());
				prev_underscore = false;
			}
		} else if !prev_underscore {
			out.push('_');
			prev_underscore = true;
		}
	}
	if out.ends_with('_') {
		out.pop();
	}
	if out.is_empty() {
		"action".to_string()
	} else {
		out
	}
}

fn create_blank_migration(args: NewArgs) -> Result<(), String> {
	let ts = unix_ts()?;
	let filename = format!("{}_{}.sql", ts, normalize_name(&args.name)?);
	let path = args.dir.join(filename);
	let body = format!(
		"-- name: {}\n-- created_at: {}\n\nBEGIN;\n\n-- write migration SQL here\n\nCOMMIT;\n",
		args.name, ts
	);
	write_file(path, body)
}

fn diff_migration(args: DiffArgs) -> Result<(), String> {
	let project_dir = resolve_project_dir(std::path::Path::new("."))?;
	let config = load_wgui_config(&project_dir)?;
	let schema_path = resolve_path_with_default(
		args.schema,
		config.schema,
		PathBuf::from("schema.wdb"),
		&project_dir,
	);
	let db_path =
		resolve_path_with_default(args.db, config.db, PathBuf::from("wgui.db"), &project_dir);

	#[cfg(not(feature = "sqlite"))]
	{
		let _ = (&schema_path, &db_path);
		Err("`wgui migrations diff` requires the `sqlite` feature".to_string())
	}
	#[cfg(feature = "sqlite")]
	{
		let sql = schema_diff_sql_from_schema_file(&schema_path, &db_path)
			.map_err(|e| format!("failed generating schema diff: {e}"))?;
		if let Some(sql) = sql {
			println!("{sql}");
		} else {
			println!("no schema changes");
		}
		Ok(())
	}
}

fn create_schema_migration(args: CreateArgs) -> Result<(), String> {
	let project_dir = resolve_project_dir(std::path::Path::new("."))?;
	let config = load_wgui_config(&project_dir)?;
	let schema_path = resolve_path_with_default(
		args.schema,
		config.schema,
		PathBuf::from("schema.wdb"),
		&project_dir,
	);
	let db_path =
		resolve_path_with_default(args.db, config.db, PathBuf::from("wgui.db"), &project_dir);
	let migrations_dir = resolve_path_with_default(
		args.dir,
		config.migrations_dir,
		PathBuf::from("migrations"),
		&project_dir,
	);

	#[cfg(not(feature = "sqlite"))]
	{
		let _ = (&schema_path, &db_path, &migrations_dir);
		Err("`wgui migrations create` requires the `sqlite` feature".to_string())
	}
	#[cfg(feature = "sqlite")]
	{
		let path = write_schema_migration_from_schema_file(
			&schema_path,
			&db_path,
			&args.name,
			&migrations_dir,
		)
		.map_err(|e| format!("failed creating migration: {e}"))?;
		if let Some(path) = path {
			println!("{}", path.display());
		} else {
			println!("no schema changes");
		}
		Ok(())
	}
}

fn compare_schemas(args: CompareArgs) -> Result<(), String> {
	let from_schema = wdb::parse_schema_file(&args.from)
		.map_err(|e| format!("failed reading --from schema: {e}"))?;
	let to_schema =
		wdb::parse_schema_file(&args.to).map_err(|e| format!("failed reading --to schema: {e}"))?;
	let from_diff = wdb::to_diff_schema(&from_schema);
	let to_diff = wdb::to_diff_schema(&to_schema);
	let ops = diff_schemas(&from_diff, &to_diff);

	if ops.is_empty() {
		println!("no schema changes");
		return Ok(());
	}

	for op in ops {
		match op {
			wgui::schema_diff::DiffOp::CreateTable { table } => {
				println!("create table {} ({})", table.name, table.columns.len());
			}
			wgui::schema_diff::DiffOp::AddColumn { table, column } => {
				println!("add column {}.{}: {}", table, column.name, column.rust_type);
			}
		}
	}
	Ok(())
}

fn migrate_dev(args: MigrateDevArgs) -> Result<(), String> {
	#[cfg(not(feature = "sqlite"))]
	{
		let _ = args;
		Err("`wgui migrate dev` requires the `sqlite` feature".to_string())
	}
	#[cfg(feature = "sqlite")]
	{
		if args.name.trim().is_empty() {
			return Err("migration name cannot be empty".to_string());
		}

		let project_dir = resolve_project_dir(&args.project_dir)?;
		let config = load_wgui_config(&project_dir)?;
		let env_path = resolve_path_with_default(
			args.env_file,
			config.env_file,
			PathBuf::from(".env"),
			&project_dir,
		);
		let schema_path = resolve_path_with_default(
			args.schema,
			config.schema,
			PathBuf::from("schema.wdb"),
			&project_dir,
		);
		let migrations_dir = resolve_path_with_default(
			args.migrations_dir,
			config.migrations_dir,
			PathBuf::from("migrations"),
			&project_dir,
		);

		if !schema_path.exists() {
			return Err(format!("schema file not found: {}", schema_path.display()));
		}
		if !env_path.exists() {
			return Err(format!(".env file not found: {}", env_path.display()));
		}

		let envs = read_env_file(&env_path)?;
		let database_url = envs
			.get("DATABASE_URL")
			.or_else(|| envs.get("WGUI_DATABASE_URL"))
			.ok_or_else(|| {
				format!(
					"DATABASE_URL not found in {} (or WGUI_DATABASE_URL)",
					env_path.display()
				)
			})?;
		let db_path = resolve_database_path(database_url, &project_dir)?;

		if let Some(parent) = db_path.parent() {
			std::fs::create_dir_all(parent)
				.map_err(|e| format!("failed creating db directory {}: {e}", parent.display()))?;
		}
		std::fs::create_dir_all(&migrations_dir).map_err(|e| {
			format!(
				"failed creating migrations dir {}: {e}",
				migrations_dir.display()
			)
		})?;

		let conn = Connection::open(&db_path)
			.map_err(|e| format!("failed opening sqlite database {}: {e}", db_path.display()))?;
		ensure_applied_migrations_table(&conn)?;

		let mut applied_any = false;
		for migration in list_sql_migrations(&migrations_dir)? {
			if is_migration_applied(&conn, &migration)? {
				continue;
			}
			let migration_path = migrations_dir.join(&migration);
			let sql = std::fs::read_to_string(&migration_path).map_err(|e| {
				format!("failed reading migration {}: {e}", migration_path.display())
			})?;
			conn.execute_batch(&sql)
				.map_err(|e| format!("failed applying migration {}: {e}", migration))?;
			mark_migration_applied(&conn, &migration)?;
			println!("applied migration {}", migration);
			applied_any = true;
		}

		let created = write_schema_migration_from_schema_file(
			&schema_path,
			&db_path,
			&args.name,
			&migrations_dir,
		)
		.map_err(|e| format!("failed creating schema migration: {e}"))?;

		if let Some(path) = created {
			let filename = path
				.file_name()
				.and_then(|s| s.to_str())
				.ok_or_else(|| format!("invalid migration file name: {}", path.display()))?
				.to_string();
			let sql = std::fs::read_to_string(&path)
				.map_err(|e| format!("failed reading migration {}: {e}", path.display()))?;
			conn.execute_batch(&sql)
				.map_err(|e| format!("failed applying migration {}: {e}", path.display()))?;
			mark_migration_applied(&conn, &filename)?;
			println!("created and applied {}", path.display());
			applied_any = true;
		}

		if !applied_any {
			println!("database is up to date");
		}
		Ok(())
	}
}

fn write_file(path: PathBuf, body: String) -> Result<(), String> {
	let parent = path
		.parent()
		.ok_or_else(|| format!("invalid migration path {}", path.display()))?;
	std::fs::create_dir_all(parent)
		.map_err(|e| format!("failed creating directory {}: {e}", parent.display()))?;
	std::fs::write(&path, body).map_err(|e| format!("failed writing {}: {e}", path.display()))?;
	println!("{}", path.display());
	Ok(())
}

fn unix_ts() -> Result<u64, String> {
	let now = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map_err(|e| format!("system clock error: {e}"))?;
	Ok(now.as_secs())
}

fn normalize_name(raw: &str) -> Result<String, String> {
	let mut out = String::new();
	for ch in raw.chars() {
		if ch.is_ascii_alphanumeric() {
			out.push(ch.to_ascii_lowercase());
		} else if ch == '-' || ch == '_' || ch == ' ' {
			if !out.ends_with('_') {
				out.push('_');
			}
		}
	}
	let out = out.trim_matches('_').to_string();
	if out.is_empty() {
		return Err("migration name must contain letters or numbers".to_string());
	}
	Ok(out)
}

fn generate_db_rs(schema: &wgui::wdb::SchemaAst, db_name: &str) -> Result<String, String> {
	if db_name.trim().is_empty() {
		return Err("db_name cannot be empty".to_string());
	}
	let mut out = String::new();
	out.push_str("use wgui::{Db, DbTable, HasId, Wdb, WguiModel};\n\n");

	let mut table_inits: Vec<(String, String)> = Vec::new();
	let mut db_fields: Vec<(String, String)> = Vec::new();

	for model in &schema.models {
		let model_name = &model.name;
		let struct_fields = model
			.fields
			.iter()
			.filter(|f| !f.attributes.iter().any(|a| a.name == "relation"))
			.collect::<Vec<_>>();

		out.push_str("#[derive(Debug, Clone, WguiModel, serde::Serialize, serde::Deserialize)]\n");
		out.push_str(&format!("pub struct {} {{\n", model_name));
		for field in &struct_fields {
			let ty = wdb_type_to_rust(&field.ty);
			out.push_str(&format!("\tpub {}: {},\n", field.name, ty));
		}
		out.push_str("}\n\n");

		let has_id_u32 = struct_fields
			.iter()
			.find(|f| f.name == "id")
			.map(|f| f.ty.name == "Int" || f.ty.name == "u32")
			.unwrap_or(false);
		if has_id_u32 {
			out.push_str(&format!("impl HasId for {} {{\n", model_name));
			out.push_str("\tfn id(&self) -> u32 {\n\t\tself.id\n\t}\n\n");
			out.push_str("\tfn set_id(&mut self, id: u32) {\n\t\tself.id = id;\n\t}\n");
			out.push_str("}\n\n");
		}

		let table_field = pluralize(&to_snake_case(model_name));
		db_fields.push((table_field.clone(), model_name.clone()));
		let init = format!("{}: db.table()", table_field);
		table_inits.push((table_field, init));
	}

	out.push_str("#[derive(Debug, Wdb)]\n");
	out.push_str(&format!("pub struct {} {{\n", db_name));
	for (field_name, model_name) in &db_fields {
		out.push_str(&format!("\tpub {}: DbTable<{}>,\n", field_name, model_name));
	}
	out.push_str("}\n\n");

	out.push_str(&format!("impl {} {{\n", db_name));
	out.push_str("\tpub fn new() -> Self {\n");
	out.push_str(&format!("\t\tlet db = Db::<{}>::new();\n", db_name));
	out.push_str("\t\tSelf {\n");
	for (_, init) in &table_inits {
		out.push_str(&format!("\t\t\t{},\n", init));
	}
	out.push_str("\t\t}\n");
	out.push_str("\t}\n");
	out.push_str("}\n");

	Ok(out)
}

fn wdb_type_to_rust(ty: &wgui::wdb::TypeAst) -> String {
	let base = match ty.name.as_str() {
		"Bool" => "bool".to_string(),
		"String" => "String".to_string(),
		"Int" => "u32".to_string(),
		"BigInt" => "i64".to_string(),
		"Float" | "Decimal" => "f64".to_string(),
		"UUID" | "DateTime" | "Json" | "Bytes" => "String".to_string(),
		other => other.to_string(),
	};
	let with_list = if ty.is_list {
		format!("Vec<{}>", base)
	} else {
		base
	};
	if ty.is_optional {
		format!("Option<{}>", with_list)
	} else {
		with_list
	}
}

fn to_snake_case(input: &str) -> String {
	let mut out = String::new();
	for (idx, ch) in input.chars().enumerate() {
		if ch.is_ascii_uppercase() {
			if idx > 0 {
				out.push('_');
			}
			out.push(ch.to_ascii_lowercase());
		} else {
			out.push(ch);
		}
	}
	out
}

fn pluralize(singular: &str) -> String {
	if singular.ends_with('s') {
		format!("{}es", singular)
	} else {
		format!("{}s", singular)
	}
}

#[cfg(feature = "sqlite")]
fn read_env_file(path: &std::path::Path) -> Result<HashMap<String, String>, String> {
	let raw = std::fs::read_to_string(path)
		.map_err(|e| format!("failed reading env file {}: {e}", path.display()))?;
	let mut out = HashMap::new();
	for line in raw.lines() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		let Some((k, v)) = line.split_once('=') else {
			continue;
		};
		let key = k.trim().to_string();
		let mut value = v.trim().to_string();
		if (value.starts_with('"') && value.ends_with('"'))
			|| (value.starts_with('\'') && value.ends_with('\''))
		{
			value = value[1..value.len() - 1].to_string();
		}
		out.insert(key, value);
	}
	Ok(out)
}

#[cfg(feature = "sqlite")]
fn resolve_database_path(url: &str, project_dir: &std::path::Path) -> Result<PathBuf, String> {
	if let Some(rest) = url.strip_prefix("sqlite://") {
		if rest.starts_with('/') {
			return Ok(PathBuf::from(rest));
		}
		return Ok(project_dir.join(rest));
	}
	if let Some(rest) = url.strip_prefix("sqlite:") {
		if rest == ":memory:" {
			return Err("sqlite :memory: is not supported for `migrate dev`".to_string());
		}
		if rest.starts_with('/') {
			return Ok(PathBuf::from(rest));
		}
		return Ok(project_dir.join(rest));
	}
	if url.contains("://") {
		let scheme = url.split("://").next().unwrap_or("unknown");
		return Err(format!(
			"database scheme `{scheme}` is not supported yet; currently only sqlite URLs are supported"
		));
	}
	Ok(project_dir.join(url))
}

#[cfg(feature = "sqlite")]
fn ensure_applied_migrations_table(conn: &Connection) -> Result<(), String> {
	conn.execute(
		"CREATE TABLE IF NOT EXISTS _wgui_migrations (\n\
\tfilename TEXT PRIMARY KEY,\n\
\tapplied_at INTEGER NOT NULL\n\
)",
		[],
	)
	.map_err(|e| format!("failed creating _wgui_migrations: {e}"))?;
	Ok(())
}

#[cfg(feature = "sqlite")]
fn is_migration_applied(conn: &Connection, filename: &str) -> Result<bool, String> {
	let found: Option<String> = conn
		.query_row(
			"SELECT filename FROM _wgui_migrations WHERE filename = ?1",
			params![filename],
			|row| row.get(0),
		)
		.optional()
		.map_err(|e| format!("failed checking migration state for {filename}: {e}"))?;
	Ok(found.is_some())
}

#[cfg(feature = "sqlite")]
fn mark_migration_applied(conn: &Connection, filename: &str) -> Result<(), String> {
	conn.execute(
		"INSERT OR REPLACE INTO _wgui_migrations (filename, applied_at) VALUES (?1, unixepoch())",
		params![filename],
	)
	.map_err(|e| format!("failed marking migration {filename} as applied: {e}"))?;
	Ok(())
}

#[cfg(feature = "sqlite")]
fn list_sql_migrations(dir: &std::path::Path) -> Result<Vec<String>, String> {
	if !dir.exists() {
		return Ok(Vec::new());
	}
	let mut files = Vec::new();
	for entry in
		std::fs::read_dir(dir).map_err(|e| format!("failed reading {}: {e}", dir.display()))?
	{
		let entry = entry.map_err(|e| format!("failed reading dir entry: {e}"))?;
		let path = entry.path();
		if path.extension().and_then(|s| s.to_str()) != Some("sql") {
			continue;
		}
		let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
			continue;
		};
		files.push(name.to_string());
	}
	files.sort();
	Ok(files)
}
