use tonic::transport::Server;
use tracing::Level;

use structopt::StructOpt;
use libp2p::{Multiaddr, PeerId};
use libp2p::multiaddr::Protocol;
use crate::tonic_responder::message_storer::MessageStorer;
use crate::message_storage::ServerAddressType;
use crate::tonic_responder::location::LocationService;
use crate::db::{MessageDb, MessageStoreDb};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

pub(crate) mod message_storage {
	include!(concat!(env!("OUT_DIR"), "/messagestorage.rs"));
}

mod config;
mod db;
mod tonic_responder;
mod network;

#[derive(Debug, StructOpt)]
#[structopt(name = "libp2p test")]
struct Opt {
	#[structopt(long)]
	ex: SocketAddr,

	#[structopt(long)]
	listen_address: Option<Multiaddr>,

	#[structopt(long)]
	clear_address: String,

	#[structopt(long)]
	db_path: PathBuf,
}

#[tokio::main]
async fn main() {
	let collector = tracing_subscriber::fmt()
		.with_max_level(Level::DEBUG)
		.finish();
	tracing::subscriber::set_global_default(collector)
		.expect("Something fucked up during setting up collector");

	let opt = Opt::from_args();

	let (mut client, mut event_loop) = network::new().await.unwrap();

	tokio::spawn(async move {
		event_loop.run().await
	});

	match opt.listen_address {
		Some(addr) => client
			.start_listening(addr)
			.await
			.expect("Listening not to fail"),
		None => client
			.start_listening("/ip4/0.0.0.0/tcp/0".parse().unwrap())
			.await
			.expect("Listening not to fail."),
	};


	if let Some(addr) = std::env::var_os("PEER") {
		let addr = Multiaddr::from_str(addr.to_str().unwrap()).expect("Couldn't pares multiaddr");
		let peer_id = match addr.iter().last() {
			Some(Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Vaild hash."),
			_ => panic!("Expect peer multiaddr to contain peer ID.")
		};
		client
			.dial(peer_id, addr)
			.await
			.expect("Dial to succeed");
	}

	let clear_addr: String = opt.clear_address;

	let client_clone = client.clone();

	let _ = tokio::spawn( async move {
		client_clone.put_clear_addr(ServerAddressType::Clear, clear_addr).await;
		println!("Successfully Put Clear Address");
	});


	let db = MessageDb::new(&opt.db_path).unwrap();

	let addr = opt.ex;
	let greeter = MessageStorer::new(db, client.clone());

	let locer = LocationService::new(client);

	println!("Storer listening on {}", addr);

	let svc = crate::message_storage::message_storage_server::MessageStorageServer::new(greeter);
	let loc = crate::message_storage::location_server::LocationServer::new(locer);
	Server::builder()
		.add_service(svc)
		.add_service(loc)
		.serve(addr)
		.await
		.unwrap();
}