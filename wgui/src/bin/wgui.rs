#[cfg(feature = "sqlite")]
use std::collections::HashMap;
#[cfg(feature = "sqlite")]
use std::path::Path;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
#[cfg(feature = "sqlite")]
use rusqlite::{params, Connection, OptionalExtension};
use wgui::{schema_diff::diff_schemas, wdb};
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

#[derive(Args, Debug)]
struct NewArgs {
	name: String,
	#[arg(long, default_value = "migrations")]
	dir: PathBuf,
}

#[derive(Args, Debug)]
struct DiffArgs {
	#[arg(long, default_value = "schema.wdb")]
	schema: PathBuf,
	#[arg(long, default_value = "wgui.db")]
	db: PathBuf,
}

#[derive(Args, Debug)]
struct CreateArgs {
	name: String,
	#[arg(long, default_value = "schema.wdb")]
	schema: PathBuf,
	#[arg(long, default_value = "wgui.db")]
	db: PathBuf,
	#[arg(long, default_value = "migrations")]
	dir: PathBuf,
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
	#[arg(default_value = ".")]
	project_dir: PathBuf,
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
	#[cfg(not(feature = "sqlite"))]
	{
		let _ = args;
		Err("`wgui migrations diff` requires the `sqlite` feature".to_string())
	}
	#[cfg(feature = "sqlite")]
	{
		let sql = schema_diff_sql_from_schema_file(&args.schema, &args.db)
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
	#[cfg(not(feature = "sqlite"))]
	{
		let _ = args;
		Err("`wgui migrations create` requires the `sqlite` feature".to_string())
	}
	#[cfg(feature = "sqlite")]
	{
		let path =
			write_schema_migration_from_schema_file(&args.schema, &args.db, &args.name, &args.dir)
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

		let project_dir = std::fs::canonicalize(&args.project_dir).map_err(|e| {
			format!(
				"failed to resolve project dir {}: {e}",
				args.project_dir.display()
			)
		})?;
		let env_path = project_dir.join(".env");
		let schema_path = project_dir.join("schema.wdb");
		let migrations_dir = project_dir.join("migrations");

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
			let sql = std::fs::read_to_string(migrations_dir.join(&migration)).map_err(|e| {
				format!(
					"failed reading migration {}: {e}",
					migrations_dir.join(&migration).display()
				)
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

#[cfg(feature = "sqlite")]
fn read_env_file(path: &Path) -> Result<HashMap<String, String>, String> {
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
fn resolve_database_path(url: &str, project_dir: &Path) -> Result<PathBuf, String> {
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
		"CREATE TABLE IF NOT EXISTS _wgui_applied_migrations (\n\
\tfilename TEXT PRIMARY KEY,\n\
\tapplied_at INTEGER NOT NULL\n\
)",
		[],
	)
	.map_err(|e| format!("failed creating _wgui_applied_migrations: {e}"))?;
	Ok(())
}

#[cfg(feature = "sqlite")]
fn is_migration_applied(conn: &Connection, filename: &str) -> Result<bool, String> {
	let found: Option<String> = conn
		.query_row(
			"SELECT filename FROM _wgui_applied_migrations WHERE filename = ?1",
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
		"INSERT OR REPLACE INTO _wgui_applied_migrations (filename, applied_at) VALUES (?1, unixepoch())",
		params![filename],
	)
	.map_err(|e| format!("failed marking migration {filename} as applied: {e}"))?;
	Ok(())
}

#[cfg(feature = "sqlite")]
fn list_sql_migrations(dir: &Path) -> Result<Vec<String>, String> {
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
