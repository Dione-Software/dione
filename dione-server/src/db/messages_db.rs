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
	pub fn establish_connection() -> MessagesDb {
		let database_path = env::var("DATABASE_URL")
			.expect("DATABASE_URL must be set");
		
		let database_url = format!("dione-server/{}", database_path);

		trace!("Opening database at {}", database_url);

		let manager = ConnectionManager::<SqliteConnection>::new(&database_url);

		let pool = Pool::builder()
			.build(manager)
			.expect("Error creating the db connection pool");

		MessagesDb {
			pool
		}
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
