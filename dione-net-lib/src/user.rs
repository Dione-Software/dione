use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct User {
	pub id: Uuid
}

impl Default for User {
	fn default() -> Self {
		Self {
			id: Uuid::new_v4(),
		}
	}
}