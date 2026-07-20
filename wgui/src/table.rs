use std::sync::Mutex;

pub trait HasId {
	fn id(&self) -> u32;
	fn set_id(&mut self, id: u32);
}

#[derive(Debug)]
pub struct Table<T> {
	rows: Mutex<Vec<T>>,
	next_id: Mutex<u32>,
}

impl<T> Table<T> {
	pub fn new(rows: Vec<T>) -> Self {
		Self {
			rows: Mutex::new(rows),
			next_id: Mutex::new(1),
		}
	}

	pub fn snapshot(&self) -> Vec<T>
	where
		T: Clone,
	{
		self.rows.lock().unwrap().clone()
	}

	pub fn replace(&self, rows: Vec<T>) {
		*self.rows.lock().unwrap() = rows;
	}

	pub async fn insert(&self, row: T) {
		self.rows.lock().unwrap().push(row);
	}
}

impl<T> Table<T>
where
	T: HasId + Clone,
{
	pub fn with_ids(rows: Vec<T>) -> Self {
		let next_id = rows
			.iter()
			.map(HasId::id)
			.max()
			.unwrap_or(0)
			.saturating_add(1);
		Self {
			rows: Mutex::new(rows),
			next_id: Mutex::new(next_id),
		}
	}

	pub fn next_id(&self) -> u32 {
		let mut next_id = self.next_id.lock().unwrap();
		let id = *next_id;
		*next_id = next_id.saturating_add(1);
		id
	}

	pub async fn save(&self, mut row: T) -> T {
		if row.id() == 0 {
			row.set_id(self.next_id());
		}
		self.rows.lock().unwrap().push(row.clone());
		row
	}

	pub async fn find(&self, id: u32) -> Option<T> {
		self.rows
			.lock()
			.unwrap()
			.iter()
			.find(|row| row.id() == id)
			.cloned()
	}

	pub async fn delete(&self, id: u32) -> bool {
		let mut rows = self.rows.lock().unwrap();
		let original_len = rows.len();
		rows.retain(|row| row.id() != id);
		rows.len() != original_len
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone)]
	struct Row(u32);

	impl HasId for Row {
		fn id(&self) -> u32 {
			self.0
		}

		fn set_id(&mut self, id: u32) {
			self.0 = id;
		}
	}

	#[tokio::test]
	async fn delete_removes_only_the_requested_row() {
		let table = Table::with_ids(vec![Row(1), Row(2)]);
		assert!(table.delete(1).await);
		assert!(!table.delete(9).await);
		assert_eq!(
			table.snapshot().iter().map(HasId::id).collect::<Vec<_>>(),
			vec![2]
		);
	}
}
