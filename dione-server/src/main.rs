use tonic::transport::Server;
use tracing::Level;

use crate::tonic_responder::save_message::MessageStorer;
use std::str::FromStr;
use std::path::Path;
use crate::config::conf_ex::Conf;

pub(crate) mod message_storage {
	tonic::include_proto!("messagestorage");
}

mod tonic_responder;
mod config;

#[tokio::main]
async fn main() {
	let config: Conf = crate::config::conf_ex::Conf::from_str("dione-server/config/dev_config.toml").unwrap();

	let collector = tracing_subscriber::fmt()
		.with_max_level(Level::DEBUG)
		.finish();

	tracing::subscriber::set_global_default(collector)
		.expect("Something fucked up during setting up collector");

	let addr= config.network_con.message_storage.into();
	let greeter = MessageStorer::default();

	println!("Storer listening on {}", addr);

	let svc = crate::message_storage::message_storage_server::MessageStorageServer::new(greeter);

	Server::builder()
		.add_service(svc)
		.serve(addr)
		.await
		.unwrap();
}
