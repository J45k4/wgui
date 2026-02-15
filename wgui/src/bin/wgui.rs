use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
	if let Err(err) = run() {
		eprintln!("wgui error: {err}");
		std::process::exit(1);
	}
}

fn run() -> Result<(), String> {
	let mut args = env::args().skip(1).collect::<Vec<_>>();
	if args.is_empty() {
		return Err(help());
	}

	match args.remove(0).as_str() {
		"migrations" => run_migrations(args),
		"help" | "--help" | "-h" => Err(help()),
		other => Err(format!("unknown command `{other}`\n\n{}", help())),
	}
}

fn run_migrations(mut args: Vec<String>) -> Result<(), String> {
	if args.is_empty() {
		return Err(migrations_help());
	}

	match args.remove(0).as_str() {
		"new" => create_migration(args),
		"help" | "--help" | "-h" => Err(migrations_help()),
		other => Err(format!(
			"unknown migrations command `{other}`\n\n{}",
			migrations_help()
		)),
	}
}

fn create_migration(args: Vec<String>) -> Result<(), String> {
	if args.is_empty() {
		return Err("missing migration name\n\n".to_string() + &migrations_help());
	}

	let mut name: Option<String> = None;
	let mut dir = PathBuf::from("migrations");
	let mut i = 0usize;

	while i < args.len() {
		match args[i].as_str() {
			"--dir" => {
				let Some(path) = args.get(i + 1) else {
					return Err("missing value for --dir".to_string());
				};
				dir = PathBuf::from(path);
				i += 2;
			}
			flag if flag.starts_with("--dir=") => {
				let value = flag.trim_start_matches("--dir=");
				if value.is_empty() {
					return Err("missing value for --dir".to_string());
				}
				dir = PathBuf::from(value);
				i += 1;
			}
			other if other.starts_with('-') => {
				return Err(format!("unknown option `{other}`"));
			}
			raw_name => {
				if name.is_some() {
					return Err("only one migration name is allowed".to_string());
				}
				name = Some(raw_name.to_string());
				i += 1;
			}
		}
	}

	let name = name.ok_or_else(|| "missing migration name".to_string())?;
	let normalized = normalize_name(&name)?;
	let timestamp = unix_ts()?;
	let filename = format!("{timestamp}_{normalized}.sql");
	let path = dir.join(filename);

	ensure_parent_exists(&path)?;
	let body = migration_template(&name, &timestamp);
	fs::write(&path, body).map_err(|e| format!("failed writing {}: {e}", path.display()))?;

	println!("{}", path.display());
	Ok(())
}

fn unix_ts() -> Result<u64, String> {
	let now = SystemTime::now()
		.duration_since(UNIX_EPOCH)
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

fn ensure_parent_exists(path: &Path) -> Result<(), String> {
	let parent = path
		.parent()
		.ok_or_else(|| format!("invalid migration path {}", path.display()))?;
	fs::create_dir_all(parent)
		.map_err(|e| format!("failed creating directory {}: {e}", parent.display()))
}

fn migration_template(name: &str, timestamp: &u64) -> String {
	format!(
		"-- name: {name}\n-- created_at: {timestamp}\n\nBEGIN;\n\n-- write migration SQL here\n\nCOMMIT;\n"
	)
}

fn help() -> String {
	"wgui

Usage:
  wgui migrations <command> [options]

Commands:
  migrations new <name> [--dir <path>]
"
	.to_string()
}

fn migrations_help() -> String {
	"wgui migrations

Usage:
  wgui migrations new <name> [--dir <path>]

Examples:
  wgui migrations new add_users_table
  wgui migrations new add_channel_topic --dir examples/puppychat/migrations
"
	.to_string()
}
