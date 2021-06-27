#[macro_use]
extern crate diesel;

use tonic::transport::Server;
use tracing::Level;

use crate::config::conf_ex::Conf;
use crate::db::messages_db::Messages;

pub(crate) mod message_storage {
	include!(concat!(env!("OUT_DIR"), "/messagestorage.rs"));
}

mod config;
mod db;

#[tokio::main]
async fn main() {
	let config: Conf = crate::config::conf_ex::Conf::from_str("dione-server/config/dev_config.toml").unwrap();
	dotenv::from_path("dione-server/.env");

	let collector = tracing_subscriber::fmt()
		.with_max_level(Level::DEBUG)
		.finish();

	tracing::subscriber::set_global_default(collector)
		.expect("Something fucked up during setting up collector");
	
	let db = Messages::establish_connection();
	
	let addr = config.network_con.message_storage.into();
	let greeter = MessageStorer::default();

	println!("Storer listening on {}", addr);

	let svc = crate::message_storage::message_storage_server::MessageStorageServer::new(greeter);

	Server::builder()
		.add_service(svc)
		.serve(addr)
		.await
		.unwrap();
}
