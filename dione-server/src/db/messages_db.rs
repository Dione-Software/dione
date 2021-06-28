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
		let database_path = env::var("DATABASE_URL")
			.expect("DATABASE_URL must be set");
		
		let database_url = format!("dione-server/{}", database_path);
		
		trace!("Opening database at {}", database_url);
		let conn = SqliteConnection::establish(&database_url)
			.expect(&format!("Error connecting to {}", database_url));
		
		Messages {
			conn
		}
	}
	
	pub fn save_message<'a>(&self, id: &'a i32, content: &'a Vec<u8>, address: &'a Vec<u8>) -> Message {
		let new_message = NewMessage {
			id,
			content,
			address
		};
		
		trace!("Saving new message");
		diesel::insert_into(messages::table)
			.values(&new_message)
			.execute(&self.conn)
			.expect("Error saving message");
		
		messages::table
			.filter(messages::id.eq(id))
			.first(&self.conn)
			.expect("Expected just inserted message to still be in the db")
	}
	
	pub fn get_message_by_id(&self, id: i32) -> QueryResult<Message> {
		messages::table
			.filter(messages::id.eq(id))
			.first(&self.conn)
	}
	
	pub fn get_message(&self, address: Vec<u8>) -> QueryResult<Vec<Message>> {
		messages::table
			.filter(messages::address.eq(address))
			.load::<Message>(&self.conn)
	}
}
