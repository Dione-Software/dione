#[macro_use]
extern crate diesel;

mod config;
mod db;
mod tonic_responder;

use tonic::transport::Server;
use tracing::Level;

use crate::config::conf_ex::Conf;
use crate::db::messages_db::MessagesDb;
use prost::alloc::str::FromStr;
use crate::tonic_responder::message_storer::MessageStorer;

pub(crate) mod message_storage {
	include!(concat!(env!("OUT_DIR"), "/messagestorage.rs"));
}



#[tokio::main]
async fn main() {
	let config: Conf = crate::config::conf_ex::Conf::from_str("dione-server/config/dev_config.toml").unwrap();
	dotenv::from_path("dione-server/.env")
		.expect("Error opening config file");

	let collector = tracing_subscriber::fmt()
		.with_max_level(Level::DEBUG)
		.finish();

	tracing::subscriber::set_global_default(collector)
		.expect("Something fucked up during setting up collector");

	let db = MessagesDb::establish_connection();

	let addr = config.network_con.message_storage.into();
	let greeter = MessageStorer::new(db);

	println!("Storer listening on {}", addr);

	let svc = crate::message_storage::message_storage_server::MessageStorageServer::new(greeter);

	Server::builder()
		.add_service(svc)
		.serve(addr)
		.await
		.unwrap();
}
