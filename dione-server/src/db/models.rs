use crate::db::schema::messages;

#[derive(Queryable)]
pub struct Message {
	pub id: i32,
	pub content: Vec<u8>,
	pub address: Vec<u8>
}

#[derive(Insertable)]
#[table_name="messages"]
pub struct NewMessage {
	pub content: Vec<u8>,
	pub address: Vec<u8>
}