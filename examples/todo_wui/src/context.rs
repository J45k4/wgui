use crate::TodoState;

#[derive(Debug, Default)]
pub struct SharedContext {
	pub state: TodoState,
}
