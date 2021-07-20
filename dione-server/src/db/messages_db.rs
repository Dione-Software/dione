use std::env;
use diesel::*;
use crate::db::models::{Message, NewMessage};
use tracing::*;
use crate::db::schema::messages;
use diesel::r2d2::{ConnectionManager, Pool};
use tokio_diesel::{AsyncRunQueryDsl, AsyncResult};
use std::fmt::{Debug, Formatter};


pub struct MessagesDb {
	pool: Pool<ConnectionManager<SqliteConnection>>
}

impl Debug for MessagesDb {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", self.pool.state())
	}
}

impl MessagesDb {
	pub fn new<S: Into<String>>(database_url: S) -> Self {
		let manager = ConnectionManager::<SqliteConnection>::new(database_url);
		let pool = Pool::builder()
			.build(manager)
			.expect("Error creating the db connection pool");

		Self {
			pool
		}
	}

	pub fn establish_connection() -> MessagesDb {
		let database_path = env::var("DATABASE_URL")
			.expect("DATABASE_URL must be set");
		
		let database_url = format!("dione-server/{}", database_path);

		trace!("Opening database at {}", database_url);

		Self::new(database_url)
	}

	#[cfg(test)]
	pub fn test_connection(path: &String) -> Self {
		let db = Self::new(path);

		diesel_migrations::run_pending_migrations(&db.pool.get().expect("Error getting connection while running migrations"))
			.expect("Error running migrations");
		db
	}

	#[cfg(test)]
	pub async fn destroy_test_connection(path: &String) {
		tokio::fs::remove_file(path).await.unwrap();
	}
	
	pub async fn save_message<'a>(&self, content: Vec<u8>, address: Vec<u8>) -> Message {
		let new_message = NewMessage {
			content,
			address: address.clone()
		};
		
		trace!("Saving new message");
		diesel::insert_into(messages::table)
			.values(new_message)
			.execute_async(&self.pool)
			.await
			.expect("Error inserting into Db");
		
		messages::table
			.filter(messages::address.eq(address))
			.first_async(&self.pool)
			.await
			.expect("Error getting the message from the db")
	}
	
	pub async fn get_message(&self, address: Vec<u8>) -> AsyncResult<Vec<Message>> {
		messages::table
			.filter(messages::address.eq(address))
			.load_async::<Message>(&self.pool)
			.await
	}
}

#[tokio::test]
async fn test_db_test() {
	let _message_db = MessagesDb::test_connection(&"test_db_test.sqlite3".to_string());
	MessagesDb::destroy_test_connection(&"test_db_test.sqlite3".to_string()).await;
	assert!(Path::new("test_db.sqlite3").exists().not())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn save_message() {
	let test_db_path = String::from("test_save_message.sqlite3");
	let test_db = MessagesDb::test_connection(&test_db_path);
	let content = b"This is the content of message".to_vec();
	let address = b"addressfield".to_vec();
	test_db.save_message(content, address).await;
	MessagesDb::destroy_test_connection(&test_db_path).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn get_message() {
	let test_db_path = String::from("test_get_message.sqilte3");
	let test_db = MessagesDb::test_connection(&test_db_path);
	let content = b"This is the content of message".to_vec();
	let address = b"addressfield".to_vec();
	test_db.save_message(content.clone(), address.clone()).await;
	let mut ret_content = test_db
		.get_message(address.clone())
		.await
		.unwrap();
	let ret = ret_content.pop().unwrap();
	MessagesDb::destroy_test_connection(&test_db_path).await;
	assert!(ret.address == address && ret.content == content)
}
