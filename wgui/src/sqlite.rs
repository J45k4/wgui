use crate::table::HasId;
use crate::wui::runtime::{WdbModel, WdbSchema};
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
		Ok(Self {
			conn: Arc::new(Mutex::new(conn)),
		})
	}

	pub fn in_memory() -> Result<Self> {
		let conn = Connection::open_in_memory().context("failed to open in-memory sqlite db")?;
		Ok(Self {
			conn: Arc::new(Mutex::new(conn)),
		})
	}

	pub fn register_model<M: WdbModel>(&self) -> Result<()> {
		let schema = M::schema();
		let table = sql_identifier(schema.model)?;
		let sql = format!(
			"CREATE TABLE IF NOT EXISTS \"{}\" (id INTEGER PRIMARY KEY AUTOINCREMENT, json TEXT NOT NULL)",
			table
		);
		let conn = self.conn.lock().unwrap();
		conn.execute(&sql, [])
			.with_context(|| format!("failed to create sqlite table {}", table))?;
		Ok(())
	}

	pub fn register_schema<S: WdbSchema>(&self) -> Result<()> {
		for model in S::schema() {
			let table = sql_identifier(model.model)?;
			let sql = format!(
				"CREATE TABLE IF NOT EXISTS \"{}\" (id INTEGER PRIMARY KEY AUTOINCREMENT, json TEXT NOT NULL)",
				table
			);
			let conn = self.conn.lock().unwrap();
			conn.execute(&sql, [])
				.with_context(|| format!("failed to create sqlite table {}", table))?;
		}
		Ok(())
	}

	pub fn table<M: WdbModel>(&self) -> Result<SqliteTable<M>> {
		self.register_model::<M>()?;
		Ok(SqliteTable {
			conn: self.conn.clone(),
			table_name: sql_identifier(M::schema().model)?,
			_marker: PhantomData,
		})
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
	pub fn snapshot_sync(&self) -> Result<Vec<T>> {
		let sql = format!("SELECT json FROM \"{}\" ORDER BY id", self.table_name);
		let conn = self.conn.lock().unwrap();
		let mut stmt = conn
			.prepare(&sql)
			.with_context(|| format!("failed to prepare snapshot query for {}", self.table_name))?;
		let rows = stmt
			.query_map([], |row| row.get::<_, String>(0))
			.with_context(|| format!("failed to query snapshot for {}", self.table_name))?;
		let mut out = Vec::new();
		for row in rows {
			let raw = row.with_context(|| format!("failed to read row for {}", self.table_name))?;
			let item: T = serde_json::from_str(&raw)
				.with_context(|| format!("failed to deserialize row in {}", self.table_name))?;
			out.push(item);
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
		let insert_sql = format!("INSERT INTO \"{}\" (json) VALUES (?1)", self.table_name);
		{
			let mut stmt = tx
				.prepare(&insert_sql)
				.with_context(|| format!("failed to prepare insert for {}", self.table_name))?;
			for row in rows {
				let json = serde_json::to_string(&row).with_context(|| {
					format!("failed to serialize row for table {}", self.table_name)
				})?;
				stmt.execute(params![json]).with_context(|| {
					format!("failed to insert row while replacing {}", self.table_name)
				})?;
			}
		}
		tx.commit().context("failed to commit sqlite transaction")?;
		Ok(())
	}

	pub async fn replace(&self, rows: Vec<T>) -> Result<()> {
		self.replace_sync(rows)
	}

	pub async fn insert(&self, row: T) -> Result<()> {
		let sql = format!("INSERT INTO \"{}\" (json) VALUES (?1)", self.table_name);
		let json = serde_json::to_string(&row)
			.with_context(|| format!("failed to serialize row for table {}", self.table_name))?;
		let conn = self.conn.lock().unwrap();
		conn.execute(&sql, params![json])
			.with_context(|| format!("failed to insert row into {}", self.table_name))?;
		Ok(())
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
		let sql = format!("SELECT json FROM \"{}\" WHERE id = ?1", self.table_name);
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
		let raw: String = row
			.get(0)
			.with_context(|| format!("failed to read find result in {}", self.table_name))?;
		let value = serde_json::from_str(&raw)
			.with_context(|| format!("failed to deserialize row in {}", self.table_name))?;
		Ok(Some(value))
	}

	pub async fn save(&self, mut row: T) -> Result<T> {
		let mut conn = self.conn.lock().unwrap();
		let tx = conn
			.transaction()
			.context("failed to start sqlite transaction")?;
		if row.id() == 0 {
			let insert_sql = format!("INSERT INTO \"{}\" (json) VALUES (?1)", self.table_name);
			tx.execute(&insert_sql, params!["{}"]).with_context(|| {
				format!("failed to insert placeholder row into {}", self.table_name)
			})?;
			let inserted = tx.last_insert_rowid();
			let inserted = u32::try_from(inserted).context("sqlite id overflowed u32")?;
			row.set_id(inserted);
		}
		let upsert_sql = format!(
			"INSERT INTO \"{}\" (id, json) VALUES (?1, ?2) \
			 ON CONFLICT(id) DO UPDATE SET json = excluded.json",
			self.table_name
		);
		let json = serde_json::to_string(&row)
			.with_context(|| format!("failed to serialize row for {}", self.table_name))?;
		tx.execute(&upsert_sql, params![row.id(), json])
			.with_context(|| format!("failed to upsert row in {}", self.table_name))?;
		tx.commit().context("failed to commit sqlite transaction")?;
		Ok(row)
	}
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

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone, serde::Serialize, serde::Deserialize, wui_derive::WguiModel)]
	struct SqliteTodo {
		id: u32,
		title: String,
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
			})
			.await
			.expect("save todo");
		assert_eq!(saved.id, 1);

		let fetched = table.find(saved.id).await.expect("find todo").expect("row");
		assert_eq!(fetched.title, "first");
	}
}
