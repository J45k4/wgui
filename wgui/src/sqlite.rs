use crate::table::HasId;
use crate::wui::runtime::{WdbModel, WdbModelSchema, WdbSchema};
use anyhow::{anyhow, Context, Result};
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct SqliteDb {
	conn: Arc<Mutex<Connection>>,
}

#[derive(Clone)]
pub struct SQLiteDB<S: WdbSchema> {
	inner: SqliteDb,
	_schema: PhantomData<S>,
}

pub type SQLLiteDB<S> = SQLiteDB<S>;

pub trait SchemaMigrations: WdbSchema {
	fn migration_sql<P: AsRef<Path>>(db_path: P) -> Result<Option<String>>
	where
		Self: Sized,
	{
		schema_diff_sql::<Self, _>(db_path)
	}

	fn create_migration<P: AsRef<Path>, Q: AsRef<Path>>(
		db_path: P,
		name: &str,
		dir: Q,
	) -> Result<Option<PathBuf>>
	where
		Self: Sized,
	{
		write_schema_migration::<Self, _, _>(db_path, name, dir)
	}
}

impl<T: WdbSchema> SchemaMigrations for T {}

pub fn schema_diff_sql<S, P>(db_path: P) -> Result<Option<String>>
where
	S: WdbSchema,
	P: AsRef<Path>,
{
	let conn =
		Connection::open(db_path).context("failed to open sqlite database for schema diff")?;
	let statements = schema_diff_statements::<S>(&conn)?;
	if statements.is_empty() {
		return Ok(None);
	}
	Ok(Some(render_migration_sql("auto_schema_diff", &statements)))
}

pub fn write_schema_migration<S, P, Q>(db_path: P, name: &str, dir: Q) -> Result<Option<PathBuf>>
where
	S: WdbSchema,
	P: AsRef<Path>,
	Q: AsRef<Path>,
{
	let conn =
		Connection::open(db_path).context("failed to open sqlite database for migration diff")?;
	let statements = schema_diff_statements::<S>(&conn)?;
	if statements.is_empty() {
		return Ok(None);
	}

	let normalized = normalize_migration_name(name)?;
	let timestamp = unix_ts()?;
	let filename = format!("{timestamp}_{normalized}.sql");
	let path = dir.as_ref().join(filename);
	let parent = path
		.parent()
		.ok_or_else(|| anyhow!("invalid migration path {}", path.display()))?;
	std::fs::create_dir_all(parent)
		.with_context(|| format!("failed creating migration dir {}", parent.display()))?;
	let sql = render_migration_sql(name, &statements);
	std::fs::write(&path, sql).with_context(|| format!("failed writing {}", path.display()))?;
	Ok(Some(path))
}

impl<S: WdbSchema> SQLiteDB<S> {
	pub fn new() -> Result<Self> {
		let path = default_db_path::<S>();
		Self::open(path)
	}

	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
		let inner = SqliteDb::open(path)?;
		inner.register_schema::<S>()?;
		Ok(Self {
			inner,
			_schema: PhantomData,
		})
	}

	pub fn in_memory() -> Result<Self> {
		let inner = SqliteDb::in_memory()?;
		inner.register_schema::<S>()?;
		Ok(Self {
			inner,
			_schema: PhantomData,
		})
	}

	pub fn db(&self) -> &SqliteDb {
		&self.inner
	}

	pub fn table<M: WdbModel>(&self) -> Result<SqliteTable<M>> {
		self.inner.table::<M>()
	}
}

impl SqliteDb {
	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
		let conn = Connection::open(path).context("failed to open sqlite database")?;
		init_migrations_table(&conn)?;
		Ok(Self {
			conn: Arc::new(Mutex::new(conn)),
		})
	}

	pub fn in_memory() -> Result<Self> {
		let conn = Connection::open_in_memory().context("failed to open in-memory sqlite db")?;
		init_migrations_table(&conn)?;
		Ok(Self {
			conn: Arc::new(Mutex::new(conn)),
		})
	}

	pub fn register_model<M: WdbModel>(&self) -> Result<()> {
		self.ensure_model_table(&M::schema())
	}

	pub fn register_schema<S: WdbSchema>(&self) -> Result<()> {
		for model in S::schema() {
			self.ensure_model_table(&model)?;
		}
		Ok(())
	}

	pub fn table<M: WdbModel>(&self) -> Result<SqliteTable<M>> {
		let schema = M::schema();
		self.ensure_model_table(&schema)?;
		Ok(SqliteTable {
			conn: self.conn.clone(),
			table_name: sql_identifier(schema.model)?,
			_marker: PhantomData,
		})
	}

	fn ensure_model_table(&self, schema: &WdbModelSchema) -> Result<()> {
		let table = sql_identifier(schema.model)?;
		let columns = model_columns(schema)?;
		let create_suffix = columns
			.iter()
			.map(|(name, rust_type)| format!(", \"{}\" {}", name, sql_affinity(rust_type)))
			.collect::<String>();
		let create_sql = format!(
			"CREATE TABLE IF NOT EXISTS \"{}\" (id INTEGER PRIMARY KEY AUTOINCREMENT{})",
			table, create_suffix
		);

		let conn = self.conn.lock().unwrap();
		conn.execute(&create_sql, [])
			.with_context(|| format!("failed to create sqlite table {}", table))?;

		let pragma = format!("PRAGMA table_info(\"{}\")", table);
		let mut stmt = conn
			.prepare(&pragma)
			.with_context(|| format!("failed to inspect table {}", table))?;
		let existing_iter = stmt
			.query_map([], |row| row.get::<_, String>(1))
			.with_context(|| format!("failed reading table info for {}", table))?;
		let mut existing = HashSet::new();
		for name in existing_iter {
			existing
				.insert(name.with_context(|| format!("failed parsing table info for {}", table))?);
		}

		for (name, rust_type) in &columns {
			if existing.contains(name) {
				continue;
			}
			let alter = format!(
				"ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}",
				table,
				name,
				sql_affinity(rust_type)
			);
			conn.execute(&alter, [])
				.with_context(|| format!("failed migration add-column {}.{}", table, name))?;
		}

		if existing.contains("json") {
			backfill_from_legacy_json(&conn, &table, schema, &columns)?;
		}

		let schema_hash = schema_hash(schema)?;
		conn.execute(
			"INSERT INTO _wgui_migrations (table_name, schema_hash, updated_at)
			 VALUES (?1, ?2, unixepoch())
			 ON CONFLICT(table_name) DO UPDATE SET
			   schema_hash = excluded.schema_hash,
			   updated_at = excluded.updated_at",
			params![table, schema_hash],
		)
		.with_context(|| format!("failed updating migration metadata for {}", table))?;
		Ok(())
	}
}

#[derive(Clone)]
pub struct SqliteTable<T> {
	conn: Arc<Mutex<Connection>>,
	table_name: String,
	_marker: PhantomData<T>,
}

impl<T> SqliteTable<T>
where
	T: WdbModel + Clone + Serialize + DeserializeOwned,
{
	pub fn row_count_sync(&self) -> Result<u64> {
		let sql = format!("SELECT COUNT(*) FROM \"{}\"", self.table_name);
		let conn = self.conn.lock().unwrap();
		let count: i64 = conn
			.query_row(&sql, [], |row| row.get(0))
			.with_context(|| format!("failed counting rows in {}", self.table_name))?;
		let count = u64::try_from(count).context("row count overflowed u64")?;
		Ok(count)
	}

	pub async fn row_count(&self) -> Result<u64> {
		self.row_count_sync()
	}

	pub fn snapshot_sync(&self) -> Result<Vec<T>> {
		let schema = T::schema();
		let cols = model_columns(&schema)?;
		let has_model_id = has_model_id(&schema);
		let select_cols = column_select_list(&cols);
		let sql = format!(
			"SELECT id{} FROM \"{}\" ORDER BY id",
			select_cols, self.table_name
		);
		let conn = self.conn.lock().unwrap();
		let mut stmt = conn
			.prepare(&sql)
			.with_context(|| format!("failed to prepare snapshot query for {}", self.table_name))?;
		let mut rows = stmt
			.query([])
			.with_context(|| format!("failed to query snapshot for {}", self.table_name))?;
		let mut out = Vec::new();
		while let Some(row) = rows.next().context("failed to iterate sqlite rows")? {
			out.push(row_to_model::<T>(row, &schema, &cols, has_model_id)?);
		}
		Ok(out)
	}

	pub async fn snapshot(&self) -> Result<Vec<T>> {
		self.snapshot_sync()
	}

	pub fn replace_sync(&self, rows: Vec<T>) -> Result<()> {
		let delete_sql = format!("DELETE FROM \"{}\"", self.table_name);
		let mut conn = self.conn.lock().unwrap();
		let tx = conn
			.transaction()
			.context("failed to start sqlite transaction")?;
		tx.execute(&delete_sql, [])
			.with_context(|| format!("failed to clear table {}", self.table_name))?;
		drop(tx);
		drop(conn);
		for row in rows {
			self.insert_sync(row)?;
		}
		Ok(())
	}

	pub async fn replace(&self, rows: Vec<T>) -> Result<()> {
		self.replace_sync(rows)
	}

	pub fn insert_sync(&self, row: T) -> Result<()> {
		let schema = T::schema();
		let cols = model_columns(&schema)?;
		let object = to_object(&row)?;
		if cols.is_empty() {
			let sql = format!("INSERT INTO \"{}\" DEFAULT VALUES", self.table_name);
			let conn = self.conn.lock().unwrap();
			conn.execute(&sql, [])
				.with_context(|| format!("failed to insert row into {}", self.table_name))?;
			return Ok(());
		}

		let col_names = cols
			.iter()
			.map(|(name, _)| format!("\"{}\"", name))
			.collect::<Vec<_>>()
			.join(", ");
		let placeholders = (1..=cols.len())
			.map(|i| format!("?{}", i))
			.collect::<Vec<_>>()
			.join(", ");
		let sql = format!(
			"INSERT INTO \"{}\" ({}) VALUES ({})",
			self.table_name, col_names, placeholders
		);
		let values = cols
			.iter()
			.map(|(name, rust_type)| json_to_sql_value(object.get(name), rust_type))
			.collect::<Vec<_>>();
		let conn = self.conn.lock().unwrap();
		conn.execute(&sql, params_from_iter(values))
			.with_context(|| format!("failed to insert row into {}", self.table_name))?;
		Ok(())
	}

	pub async fn insert(&self, row: T) -> Result<()> {
		self.insert_sync(row)
	}
}

impl<T> SqliteTable<T>
where
	T: WdbModel + HasId + Clone + Serialize + DeserializeOwned,
{
	pub fn next_id_sync(&self) -> Result<u32> {
		let sql = format!(
			"SELECT COALESCE(MAX(id), 0) + 1 FROM \"{}\"",
			self.table_name
		);
		let conn = self.conn.lock().unwrap();
		let id: i64 = conn
			.query_row(&sql, [], |row| row.get(0))
			.with_context(|| format!("failed to read next id for {}", self.table_name))?;
		let id = u32::try_from(id).context("sqlite id overflowed u32")?;
		Ok(id)
	}

	pub async fn next_id(&self) -> Result<u32> {
		self.next_id_sync()
	}

	pub async fn find(&self, id: u32) -> Result<Option<T>> {
		let schema = T::schema();
		let cols = model_columns(&schema)?;
		let has_model_id = has_model_id(&schema);
		let select_cols = column_select_list(&cols);
		let sql = format!(
			"SELECT id{} FROM \"{}\" WHERE id = ?1",
			select_cols, self.table_name
		);
		let conn = self.conn.lock().unwrap();
		let mut stmt = conn
			.prepare(&sql)
			.with_context(|| format!("failed to prepare find query for {}", self.table_name))?;
		let mut rows = stmt
			.query(params![id])
			.with_context(|| format!("failed to run find query on {}", self.table_name))?;
		let Some(row) = rows.next().context("failed to iterate sqlite rows")? else {
			return Ok(None);
		};
		Ok(Some(row_to_model::<T>(row, &schema, &cols, has_model_id)?))
	}

	pub async fn save(&self, mut row: T) -> Result<T> {
		let schema = T::schema();
		let cols = model_columns(&schema)?;
		let mut object = to_object(&row)?;
		if row.id() == 0 {
			let sql = format!("INSERT INTO \"{}\" DEFAULT VALUES", self.table_name);
			let conn = self.conn.lock().unwrap();
			conn.execute(&sql, []).with_context(|| {
				format!("failed to insert placeholder row into {}", self.table_name)
			})?;
			let inserted = conn.last_insert_rowid();
			let inserted = u32::try_from(inserted).context("sqlite id overflowed u32")?;
			row.set_id(inserted);
			object.insert(
				"id".to_string(),
				JsonValue::Number(serde_json::Number::from(inserted)),
			);
		}

		if cols.is_empty() {
			return Ok(row);
		}

		let set_clause = cols
			.iter()
			.enumerate()
			.map(|(idx, (name, _))| format!("\"{}\" = ?{}", name, idx + 2))
			.collect::<Vec<_>>()
			.join(", ");
		let sql = format!(
			"UPDATE \"{}\" SET {} WHERE id = ?1",
			self.table_name, set_clause
		);
		let mut values = Vec::with_capacity(cols.len() + 1);
		values.push(SqlValue::Integer(i64::from(row.id())));
		for (name, rust_type) in &cols {
			values.push(json_to_sql_value(object.get(name), rust_type));
		}
		let conn = self.conn.lock().unwrap();
		conn.execute(&sql, params_from_iter(values))
			.with_context(|| format!("failed to update row in {}", self.table_name))?;
		Ok(row)
	}
}

fn init_migrations_table(conn: &Connection) -> Result<()> {
	conn.execute(
		"CREATE TABLE IF NOT EXISTS _wgui_migrations (
			table_name TEXT PRIMARY KEY,
			schema_hash TEXT NOT NULL,
			updated_at INTEGER NOT NULL
		)",
		[],
	)
	.context("failed to create _wgui_migrations table")?;
	Ok(())
}

fn schema_diff_statements<S: WdbSchema>(conn: &Connection) -> Result<Vec<String>> {
	let mut statements = Vec::new();
	if !table_exists(conn, "_wgui_migrations")? {
		statements.push(
			"CREATE TABLE IF NOT EXISTS \"_wgui_migrations\" (\n\
\ttable_name TEXT PRIMARY KEY,\n\
\tschema_hash TEXT NOT NULL,\n\
\tupdated_at INTEGER NOT NULL\n\
)"
			.to_string(),
		);
	}

	for model in S::schema() {
		statements.extend(schema_model_diff_statements(conn, &model)?);
	}

	Ok(statements)
}

fn schema_model_diff_statements(conn: &Connection, schema: &WdbModelSchema) -> Result<Vec<String>> {
	let table = sql_identifier(schema.model)?;
	let columns = model_columns(schema)?;
	let mut statements = Vec::new();

	if !table_exists(conn, &table)? {
		let create_suffix = columns
			.iter()
			.map(|(name, rust_type)| format!(", \"{}\" {}", name, sql_affinity(rust_type)))
			.collect::<String>();
		statements.push(format!(
			"CREATE TABLE IF NOT EXISTS \"{}\" (id INTEGER PRIMARY KEY AUTOINCREMENT{})",
			table, create_suffix
		));
	} else {
		let existing = table_columns(conn, &table)?;
		for (name, rust_type) in &columns {
			if existing.contains_key(name) {
				continue;
			}
			statements.push(format!(
				"ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}",
				table,
				name,
				sql_affinity(rust_type)
			));
		}

		if existing.contains_key("json") {
			for (name, _) in &columns {
				statements.push(format!(
					"UPDATE \"{}\" SET \"{}\" = COALESCE(\"{}\", json_extract(\"json\", '$.{}')) WHERE json IS NOT NULL",
					table, name, name, name
				));
			}
		}
	}

	let schema_hash = schema_hash(schema)?;
	statements.push(format!(
		"INSERT INTO \"_wgui_migrations\" (table_name, schema_hash, updated_at)\n\
VALUES ('{}', '{}', unixepoch())\n\
ON CONFLICT(table_name) DO UPDATE SET\n\
\tschema_hash = excluded.schema_hash,\n\
\tupdated_at = excluded.updated_at",
		table, schema_hash
	));
	Ok(statements)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
	let exists: Option<String> = conn
		.query_row(
			"SELECT name FROM sqlite_master WHERE type='table' AND name = ?1",
			params![table],
			|row| row.get(0),
		)
		.optional()
		.with_context(|| format!("failed to inspect sqlite table {}", table))?;
	Ok(exists.is_some())
}

fn table_columns(conn: &Connection, table: &str) -> Result<HashMap<String, String>> {
	let pragma = format!("PRAGMA table_info(\"{}\")", table);
	let mut stmt = conn
		.prepare(&pragma)
		.with_context(|| format!("failed to inspect sqlite columns for {}", table))?;
	let iter = stmt
		.query_map([], |row| {
			let name = row.get::<_, String>(1)?;
			let ty = row.get::<_, String>(2)?;
			Ok((name, ty))
		})
		.with_context(|| format!("failed reading sqlite table info for {}", table))?;
	let mut out = HashMap::new();
	for item in iter {
		let (name, ty) =
			item.with_context(|| format!("failed parsing sqlite table info for {}", table))?;
		out.insert(name, ty);
	}
	Ok(out)
}

fn render_migration_sql(name: &str, statements: &[String]) -> String {
	let timestamp = unix_ts().unwrap_or(0);
	let mut out = format!("-- name: {name}\n-- created_at: {timestamp}\n\nBEGIN;\n\n");
	for statement in statements {
		out.push_str(statement);
		out.push_str(";\n\n");
	}
	out.push_str("COMMIT;\n");
	out
}

fn normalize_migration_name(raw: &str) -> Result<String> {
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
		return Err(anyhow!("migration name must contain letters or numbers"));
	}
	Ok(out)
}

fn unix_ts() -> Result<u64> {
	let now = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock error")?;
	Ok(now.as_secs())
}

fn model_columns(schema: &WdbModelSchema) -> Result<Vec<(String, String)>> {
	let mut cols = Vec::new();
	for field in &schema.fields {
		if field.name == "id" {
			continue;
		}
		cols.push((sql_identifier(field.name)?, field.rust_type.to_string()));
	}
	Ok(cols)
}

fn has_model_id(schema: &WdbModelSchema) -> bool {
	schema.fields.iter().any(|f| f.name == "id")
}

fn schema_hash(schema: &WdbModelSchema) -> Result<String> {
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	sql_identifier(schema.model)?.hash(&mut hasher);
	for field in &schema.fields {
		field.name.hash(&mut hasher);
		field.rust_type.hash(&mut hasher);
	}
	Ok(format!("{:x}", hasher.finish()))
}

fn sql_identifier(name: &str) -> Result<String> {
	if name.is_empty() {
		return Err(anyhow!("sqlite identifier cannot be empty"));
	}
	if name
		.chars()
		.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
	{
		Ok(name.to_string())
	} else {
		Err(anyhow!(
			"invalid sqlite identifier `{}`: only [A-Za-z0-9_$] allowed",
			name
		))
	}
}

fn sql_affinity(rust_type: &str) -> &'static str {
	let ty = normalized_type(rust_type);
	match ty.as_str() {
		"bool" => "INTEGER",
		"u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize" => {
			"INTEGER"
		}
		"f32" | "f64" => "REAL",
		"String" | "&str" => "TEXT",
		_ => "TEXT",
	}
}

fn normalized_type(ty: &str) -> String {
	let mut current = ty.trim().replace(' ', "");
	loop {
		if let Some(rest) = current.strip_prefix("Option<") {
			if let Some(inner) = rest.strip_suffix('>') {
				current = inner.to_string();
				continue;
			}
		}
		return current;
	}
}

fn to_object<T: Serialize>(value: &T) -> Result<serde_json::Map<String, JsonValue>> {
	let json = serde_json::to_value(value).context("failed to serialize model")?;
	let JsonValue::Object(object) = json else {
		return Err(anyhow!("model serialization did not produce object"));
	};
	Ok(object)
}

fn json_to_sql_value(value: Option<&JsonValue>, rust_type: &str) -> SqlValue {
	let ty = normalized_type(rust_type);
	let Some(value) = value else {
		return SqlValue::Null;
	};
	if value.is_null() {
		return SqlValue::Null;
	}
	match ty.as_str() {
		"bool" => {
			let v = value
				.as_bool()
				.map(|b| if b { 1 } else { 0 })
				.or_else(|| value.as_i64())
				.unwrap_or(0);
			SqlValue::Integer(v)
		}
		"u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize" => {
			if let Some(v) = value.as_i64() {
				SqlValue::Integer(v)
			} else if let Some(v) = value.as_u64() {
				SqlValue::Integer(v as i64)
			} else {
				SqlValue::Null
			}
		}
		"f32" | "f64" => value.as_f64().map(SqlValue::Real).unwrap_or(SqlValue::Null),
		"String" | "&str" => value
			.as_str()
			.map(|s| SqlValue::Text(s.to_string()))
			.unwrap_or_else(|| SqlValue::Text(value.to_string())),
		_ => SqlValue::Text(value.to_string()),
	}
}

fn row_to_model<T>(
	row: &rusqlite::Row<'_>,
	schema: &WdbModelSchema,
	cols: &[(String, String)],
	has_model_id: bool,
) -> Result<T>
where
	T: DeserializeOwned,
{
	let mut object = serde_json::Map::new();
	if has_model_id {
		let id: i64 = row
			.get(0)
			.context("failed to read sqlite row id for model")?;
		object.insert(
			"id".to_string(),
			JsonValue::Number(serde_json::Number::from(id)),
		);
	}
	for (idx, field) in schema.fields.iter().filter(|f| f.name != "id").enumerate() {
		let col_idx = idx + 1;
		let value_ref = row
			.get_ref(col_idx)
			.with_context(|| format!("failed to read sqlite column {}", field.name))?;
		let value = sql_to_json_value(value_ref, field.rust_type)?;
		object.insert(field.name.to_string(), value);
	}
	let value = JsonValue::Object(object);
	let model = serde_json::from_value(value).with_context(|| {
		let names = cols.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>();
		format!("failed to deserialize model from columns {:?}", names)
	})?;
	Ok(model)
}

fn sql_to_json_value(value: ValueRef<'_>, rust_type: &str) -> Result<JsonValue> {
	let ty = normalized_type(rust_type);
	if matches!(value, ValueRef::Null) {
		if is_option_type(rust_type) {
			return Ok(JsonValue::Null);
		}
		return Ok(default_json_value(ty.as_str()));
	}
	let json = match (ty.as_str(), value) {
		("bool", ValueRef::Integer(v)) => JsonValue::Bool(v != 0),
		(
			"u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize",
			ValueRef::Integer(v),
		) => JsonValue::Number(serde_json::Number::from(v)),
		("f32" | "f64", ValueRef::Real(v)) => serde_json::Number::from_f64(v)
			.map(JsonValue::Number)
			.unwrap_or(JsonValue::Null),
		("String" | "&str", ValueRef::Text(v)) => JsonValue::String(
			std::str::from_utf8(v)
				.context("invalid utf8 in sqlite text")?
				.to_string(),
		),
		(_, ValueRef::Text(v)) => {
			let s = std::str::from_utf8(v).context("invalid utf8 in sqlite text")?;
			serde_json::from_str::<JsonValue>(s)
				.unwrap_or_else(|_| JsonValue::String(s.to_string()))
		}
		(_, ValueRef::Integer(v)) => JsonValue::Number(serde_json::Number::from(v)),
		(_, ValueRef::Real(v)) => serde_json::Number::from_f64(v)
			.map(JsonValue::Number)
			.unwrap_or(JsonValue::Null),
		(_, ValueRef::Blob(_)) => {
			return Err(anyhow!(
				"blob sqlite values are not supported in model decoding"
			))
		}
		_ => JsonValue::Null,
	};
	Ok(json)
}

fn column_select_list(cols: &[(String, String)]) -> String {
	if cols.is_empty() {
		String::new()
	} else {
		let joined = cols
			.iter()
			.map(|(n, _)| format!("\"{}\"", n))
			.collect::<Vec<_>>()
			.join(", ");
		format!(", {}", joined)
	}
}

fn is_option_type(ty: &str) -> bool {
	ty.trim().replace(' ', "").starts_with("Option<")
}

fn default_json_value(ty: &str) -> JsonValue {
	let norm = normalized_type(ty);
	match norm.as_str() {
		"bool" => JsonValue::Bool(false),
		"u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize" => {
			JsonValue::Number(serde_json::Number::from(0))
		}
		"f32" | "f64" => JsonValue::Number(
			serde_json::Number::from_f64(0.0).unwrap_or_else(|| serde_json::Number::from(0)),
		),
		"String" | "&str" => JsonValue::String(String::new()),
		_ if norm.starts_with("Vec<") => JsonValue::Array(Vec::new()),
		_ => JsonValue::Null,
	}
}

fn backfill_from_legacy_json(
	conn: &Connection,
	table: &str,
	_schema: &WdbModelSchema,
	columns: &[(String, String)],
) -> Result<()> {
	let select_sql = format!("SELECT id, json FROM \"{}\" WHERE json IS NOT NULL", table);
	let mut select = conn
		.prepare(&select_sql)
		.with_context(|| format!("failed to prepare legacy json scan for {}", table))?;
	let mut rows = select
		.query([])
		.with_context(|| format!("failed to query legacy json rows for {}", table))?;

	if columns.is_empty() {
		return Ok(());
	}

	let set_clause = columns
		.iter()
		.enumerate()
		.map(|(idx, (name, _))| format!("\"{}\" = COALESCE(\"{}\", ?{})", name, name, idx + 1))
		.collect::<Vec<_>>()
		.join(", ");
	let update_sql = format!(
		"UPDATE \"{}\" SET {} WHERE id = ?{}",
		table,
		set_clause,
		columns.len() + 1
	);
	let mut update = conn
		.prepare(&update_sql)
		.with_context(|| format!("failed to prepare legacy update for {}", table))?;

	while let Some(row) = rows.next().context("failed iterating legacy rows")? {
		let id: i64 = row.get(0).context("failed reading legacy row id")?;
		let raw: String = row.get(1).context("failed reading legacy json payload")?;
		let value: JsonValue = serde_json::from_str(&raw)
			.with_context(|| format!("failed parsing legacy json row in {}", table))?;
		let JsonValue::Object(object) = value else {
			continue;
		};

		let mut values = Vec::with_capacity(columns.len() + 1);
		for (name, rust_type) in columns {
			let val = object
				.get(name)
				.cloned()
				.unwrap_or_else(|| default_json_value(rust_type));
			values.push(json_to_sql_value(Some(&val), rust_type));
		}
		values.push(SqlValue::Integer(id));
		update
			.execute(params_from_iter(values))
			.with_context(|| format!("failed backfilling legacy row {} in {}", id, table))?;
	}

	Ok(())
}

fn default_db_path<S>() -> PathBuf {
	let raw = std::any::type_name::<S>();
	let mut out = String::with_capacity(raw.len());
	for ch in raw.chars() {
		if ch.is_ascii_alphanumeric() || ch == '_' {
			out.push(ch);
		} else {
			out.push('_');
		}
	}
	PathBuf::from(format!("{out}.db"))
}

pub fn default_db_path_for_schema<S: WdbSchema>() -> PathBuf {
	default_db_path::<S>()
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs;
	use std::path::PathBuf;

	#[derive(Clone, serde::Serialize, serde::Deserialize)]
	struct SqliteTodo {
		id: u32,
		title: String,
		done: bool,
	}

	impl crate::wui::runtime::WdbModel for SqliteTodo {
		fn schema() -> crate::wui::runtime::WdbModelSchema {
			crate::wui::runtime::WdbModelSchema {
				model: "SqliteTodo",
				fields: vec![
					crate::wui::runtime::WdbFieldSchema {
						name: "id",
						rust_type: "u32",
					},
					crate::wui::runtime::WdbFieldSchema {
						name: "title",
						rust_type: "String",
					},
					crate::wui::runtime::WdbFieldSchema {
						name: "done",
						rust_type: "bool",
					},
				],
			}
		}
	}

	impl HasId for SqliteTodo {
		fn id(&self) -> u32 {
			self.id
		}

		fn set_id(&mut self, id: u32) {
			self.id = id;
		}
	}

	#[tokio::test]
	async fn sqlite_table_can_save_and_find() {
		let db = SqliteDb::in_memory().expect("sqlite in-memory db");
		let table = db.table::<SqliteTodo>().expect("todo table");

		let saved = table
			.save(SqliteTodo {
				id: 0,
				title: "first".to_string(),
				done: true,
			})
			.await
			.expect("save todo");
		assert_eq!(saved.id, 1);

		let fetched = table.find(saved.id).await.expect("find todo").expect("row");
		assert_eq!(fetched.title, "first");
		assert!(fetched.done);
	}

	#[test]
	fn migrations_table_is_created() {
		let db = SqliteDb::in_memory().expect("sqlite db");
		let conn = db.conn.lock().unwrap();
		let found: Option<String> = conn
			.query_row(
				"SELECT name FROM sqlite_master WHERE type='table' AND name='_wgui_migrations'",
				[],
				|row| row.get(0),
			)
			.optional()
			.expect("query sqlite_master");
		assert_eq!(found.as_deref(), Some("_wgui_migrations"));
	}

	struct SqliteTodoSchema;
	impl WdbSchema for SqliteTodoSchema {
		fn schema() -> Vec<WdbModelSchema> {
			vec![SqliteTodo::schema()]
		}
	}

	#[test]
	fn schema_diff_creates_missing_table() {
		let db_path = temp_db_path("schema_diff_creates_missing_table");
		let sql = schema_diff_sql::<SqliteTodoSchema, _>(&db_path)
			.expect("schema diff")
			.expect("migration sql");
		assert!(sql.contains("CREATE TABLE IF NOT EXISTS \"SqliteTodo\""));
		assert!(sql.contains("INSERT INTO \"_wgui_migrations\""));
		let _ = fs::remove_file(db_path);
	}

	#[test]
	fn schema_diff_adds_missing_column() {
		let db_path = temp_db_path("schema_diff_adds_missing_column");
		let conn = Connection::open(&db_path).expect("open sqlite");
		conn.execute(
			"CREATE TABLE \"SqliteTodo\" (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT)",
			[],
		)
		.expect("create legacy table");
		drop(conn);

		let sql = schema_diff_sql::<SqliteTodoSchema, _>(&db_path)
			.expect("schema diff")
			.expect("migration sql");
		assert!(sql.contains("ALTER TABLE \"SqliteTodo\" ADD COLUMN \"done\" INTEGER"));
		let _ = fs::remove_file(db_path);
	}

	fn temp_db_path(label: &str) -> PathBuf {
		let ts = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("clock")
			.as_nanos();
		std::env::temp_dir().join(format!("wgui_{label}_{ts}.db"))
	}
}
