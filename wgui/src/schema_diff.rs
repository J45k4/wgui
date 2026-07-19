use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
	pub tables: Vec<TableSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSchema {
	pub name: String,
	pub columns: Vec<ColumnSchema>,
	pub indexes: Vec<IndexSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSchema {
	pub name: String,
	pub rust_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexSchema {
	pub name: String,
	pub columns: Vec<String>,
	pub unique: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffOp {
	CreateTable { table: TableSchema },
	AddColumn { table: String, column: ColumnSchema },
	CreateIndex { table: String, index: IndexSchema },
}

pub fn diff_schemas(current: &Schema, target: &Schema) -> Vec<DiffOp> {
	let current_map: HashMap<&str, &TableSchema> = current
		.tables
		.iter()
		.map(|t| (t.name.as_str(), t))
		.collect();

	let mut ops = Vec::new();
	for table in &target.tables {
		match current_map.get(table.name.as_str()) {
			None => ops.push(DiffOp::CreateTable {
				table: table.clone(),
			}),
			Some(existing) => {
				let existing_cols: HashMap<&str, &ColumnSchema> = existing
					.columns
					.iter()
					.map(|c| (c.name.as_str(), c))
					.collect();
				for col in &table.columns {
					if existing_cols.contains_key(col.name.as_str()) {
						continue;
					}
					ops.push(DiffOp::AddColumn {
						table: table.name.clone(),
						column: col.clone(),
					});
				}
			}
		}
	}

	for table in &target.tables {
		let existing_indexes = current_map
			.get(table.name.as_str())
			.map(|existing| {
				existing
					.indexes
					.iter()
					.map(|index| (index.name.as_str(), index))
					.collect::<HashMap<_, _>>()
			})
			.unwrap_or_default();
		for index in &table.indexes {
			if existing_indexes.contains_key(index.name.as_str()) {
				continue;
			}
			ops.push(DiffOp::CreateIndex {
				table: table.name.clone(),
				index: index.clone(),
			});
		}
	}

	ops
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn creates_new_table_when_missing() {
		let current = Schema { tables: vec![] };
		let target = Schema {
			tables: vec![TableSchema {
				name: "Message".to_string(),
				columns: vec![ColumnSchema {
					name: "body".to_string(),
					rust_type: "String".to_string(),
				}],
				indexes: vec![],
			}],
		};

		let ops = diff_schemas(&current, &target);
		assert_eq!(ops.len(), 1);
		assert!(matches!(ops[0], DiffOp::CreateTable { .. }));
	}

	#[test]
	fn adds_only_missing_columns() {
		let current = Schema {
			tables: vec![TableSchema {
				name: "Message".to_string(),
				columns: vec![ColumnSchema {
					name: "body".to_string(),
					rust_type: "String".to_string(),
				}],
				indexes: vec![],
			}],
		};
		let target = Schema {
			tables: vec![TableSchema {
				name: "Message".to_string(),
				columns: vec![
					ColumnSchema {
						name: "body".to_string(),
						rust_type: "String".to_string(),
					},
					ColumnSchema {
						name: "author".to_string(),
						rust_type: "String".to_string(),
					},
				],
				indexes: vec![],
			}],
		};

		let ops = diff_schemas(&current, &target);
		assert_eq!(ops.len(), 1);
		match &ops[0] {
			DiffOp::AddColumn { table, column } => {
				assert_eq!(table, "Message");
				assert_eq!(column.name, "author");
			}
			_ => panic!("expected add column"),
		}
	}

	#[test]
	fn creates_indexes_missing_from_current_schema() {
		let current = Schema { tables: vec![] };
		let target = Schema {
			tables: vec![TableSchema {
				name: "Message".to_string(),
				columns: vec![],
				indexes: vec![IndexSchema {
					name: "idx_message_author_time".to_string(),
					columns: vec!["author".to_string(), "time".to_string()],
					unique: false,
				}],
			}],
		};

		let ops = diff_schemas(&current, &target);
		assert!(matches!(
			ops.as_slice(),
			[DiffOp::CreateTable { .. }, DiffOp::CreateIndex { table, index }]
				if table == "Message" && index.unique == false
		));
	}
}
