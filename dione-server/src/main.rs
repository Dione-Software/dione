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
use actix_rt::{System, Arbiter};

pub(crate) mod message_storage {
	include!(concat!(env!("OUT_DIR"), "/messagestorage.rs"));
}

mod db;
mod tonic_responder;
mod network;
mod web_service;

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "Dione Server")]
struct Opt {
	#[structopt(long)]
	ex: SocketAddr,

	#[structopt(long)]
	listen_address: Option<Multiaddr>,

	#[structopt(long)]
	clear_address: Option<String>,

	#[structopt(long)]
	db_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
	std::env::set_var("RUST_LOG", "actix_web=info");

	let collector = tracing_subscriber::fmt()
		.with_max_level(Level::INFO)
		.finish();
	tracing::subscriber::set_global_default(collector)
		.expect("Something fucked up during setting up collector");

	let opt = Opt::from_args();

	let rt = tokio::runtime::Runtime::new().unwrap();

	let (client, mut event_loop) = rt.block_on( async move {
		network::new().await.unwrap()
	});

	let event_loop_handler = rt.spawn(async move {
		event_loop.run().await
	});

	let mut listen_client = client.clone();
	let opt_listen = opt.clone();

	rt.block_on(async move {
		match opt_listen.listen_address {
			Some(addr) => listen_client
				.start_listening(addr)
				.await
				.expect("Listening not to fail"),
			None => listen_client
				.start_listening("/ip4/0.0.0.0/tcp/0".parse().unwrap())
				.await
				.expect("Listening not to fail."),
		};
	});

	let mut client_dial = client.clone();

	if let Some(addr) = std::env::var_os("PEER") {
		let addr = Multiaddr::from_str(addr.to_str().unwrap()).expect("Couldn't pares multiaddr");
		let peer_id = match addr.iter().last() {
			Some(Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Vaild hash."),
			_ => panic!("Expect peer multiaddr to contain peer ID.")
		};
		rt.block_on(async move {
			client_dial
				.dial(peer_id, addr)
				.await
				.expect("Dial to succeed");
		});
	}

	let clear_addr: String = match opt.clear_address {
		Some(d) => d,
		None => {
			let host = std::env::var_os("CLEARADDRESS").expect("Either pass argument or env variable").to_str().unwrap().to_owned();
			format!("http://{}:8010", host)
		}
	};
	println!("Clear Address => {}", clear_addr);
	let client_clone = client.clone();

	let put_clear_address_handler = rt.spawn( async move {
		client_clone.put_clear_addr(ServerAddressType::Clear, clear_addr).await;
		println!("Successfully Put Clear Address");
	});

	let client_clone = client.clone();

	let _ = System::new();
	let arbiter = Arbiter::new();
	arbiter.spawn(async move {
		web_service::make_server(client_clone).await.unwrap();
	});

	let signal_handler = rt.spawn(async move {
		tokio::signal::ctrl_c().await.unwrap();
		arbiter.stop();
		put_clear_address_handler.abort();
		event_loop_handler.abort();
	});



	let db = MessageDb::new(&opt.db_path).unwrap();

	let addr = opt.ex;
	let greeter = MessageStorer::new(db, client.clone());

	let locer = LocationService::new(client);

	println!("Storer listening on {}", addr);

	let svc = crate::message_storage::message_storage_server::MessageStorageServer::new(greeter);
	let loc = crate::message_storage::location_server::LocationServer::new(locer);
	let server_handler = rt.spawn(async move {
		Server::builder()
			.add_service(svc)
			.add_service(loc)
			.serve(addr)
			.await
			.unwrap();
	});
	rt.block_on(async move {
		tokio::signal::ctrl_c().await.unwrap();
		server_handler.abort();
		signal_handler.abort();
	});
	Ok(())
}