use std::env;
use diesel::*;
use dotenv::dotenv;
use crate::db::models::{Message, NewMessage};
use tracing::*;
use crate::db::schema::messages;

pub struct Messages {
	conn: SqliteConnection
}

impl Messages {
	pub fn establish_connection() -> Messages {
		trace!("Loading .env file");
		dotenv().ok();
		
		let database_url = env::var("DATABASE_URL")
			.expect("DATABASE_URL must be set");
		
		trace!("Opening database at {}", database_url);
		let conn = SqliteConnection::establish(&database_url)
			.expect(&format!("Error connecting to {}", database_url));
		
		Messages {
			conn
		}
	}
	
	pub fn save_message<'a>(&self, id: &'a i32, content: &'a Vec<u8>) -> Message {
		let new_message = NewMessage {
			id,
			content,
		};
		
		trace!("Saving new message");
		diesel::insert_into(messages::table)
			.values(&new_message)
			.execute(&self.conn)
			.expect("Expecting new id to not be a duplicate");
		
		messages::table
			.filter(messages::id.eq(id))
			.first(&self.conn)
			.expect("Expected just inserted message to still be in the db")
	}
	
	pub fn get_message(&self, id: i32) -> QueryResult<Message> {
		messages::table
			.filter(messages::id.eq(id))
			.first(&self.conn)
	}
}
