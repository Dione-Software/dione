#[macro_use]
extern crate diesel;

use tonic::transport::Server;
use tracing::Level;

use crate::config::conf_ex::Conf;
use crate::db::messages_db::MessagesDb;
use prost::alloc::str::FromStr;
use structopt::StructOpt;
use libp2p::{Multiaddr, PeerId};
use std::path::PathBuf;
use libp2p::multiaddr::Protocol;
use tokio::time::{sleep, Duration};
use crate::tonic_responder::message_storer::MessageStorer;
use crate::message_storage::ServerAddressType;

pub(crate) mod message_storage {
	include!(concat!(env!("OUT_DIR"), "/messagestorage.rs"));
}

mod config;
mod db;
mod tonic_responder;
mod network;

#[tokio::main]
async fn main() {
	let opt = Opt::from_args();


	let (mut client, mut event_loop) = network::new().await.unwrap();
	tokio::spawn(async move {
		event_loop.run().await;
	});

	match opt.listen_address {
		Some(addr) => client.start_listening(addr)
			.await
			.expect("Listening not to fail"),
		None => client.start_listening("/ip4/0.0.0.0/tcp/0".parse().unwrap())
			.await
			.expect("Listening not to fail")
	};

	if let Some(addr) = opt.peer {
		let peer_id = match addr.iter().last() {
			Some(Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Valid hash."),
			_ => panic!("Expect peer multiaddr to contain peer_id")
		};
		client
			.dial(peer_id, addr)
			.await
			.expect("Dial to succeed");
	}

	match opt.argument {
		CliArgument::Provide { addr } => {
			client.start_providing(addr.into_bytes()).await;
		},
		CliArgument::Get { addr } => {
			let providers = client.get_providers(addr.into_bytes()).await;

			if providers.is_empty() {
				panic!("Couldn't find provider")
			}

			println!("Providers => {:?}", providers);
		},
		CliArgument::PutAddr { addr } => {
			client.put_clear_addr(ServerAddressType::Clear, addr).await;
		},
		CliArgument::GetAddr { peer_id } => {
			let bundle = client.get_clear_addr(PeerId::from_str(&peer_id).unwrap()).await.unwrap();
			println!("Bundle => {:?}", bundle);
		}
	}
	loop {
		sleep(Duration::from_secs(10)).await;
	}
}

#[derive(Debug, StructOpt)]
#[structopt(name = "libp2p test")]
struct Opt {
	#[structopt(long)]
	peer: Option<Multiaddr>,

	#[structopt(long)]
	listen_address: Option<Multiaddr>,

	#[structopt(subcommand)]
	argument: CliArgument,
}

#[derive(Debug, StructOpt)]
enum CliArgument {
	Provide {
		#[structopt(long)]
		addr: String,
	},
	Get {
		#[structopt(long)]
		addr: String,
	},
	PutAddr {
		#[structopt(long)]
		addr: String,
	},
	GetAddr {
		#[structopt(long)]
		peer_id: String,
	}
}

/*
#[tokio::main]
async fn main() {
	let config: Conf = crate::config::conf_ex::Conf::from_str("dione-server/config/dev_config.toml").unwrap();
	dotenv::from_path("dione-server/.env")
		.expect("Error opening config file");

	let (mut client, mut event_loop) = network::new().await.unwrap();

	tokio::spawn(async move {
		event_loop.run().await
	});

	let config: Conf = crate::config::conf_ex::Conf::from_str("dione-server/config/dev_config.toml").unwrap();

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
*/