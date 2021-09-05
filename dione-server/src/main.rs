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
use rustls::{ServerConfig, NoClientAuth};
use std::io::BufReader;
use std::fs::File;
use rustls::internal::pemfile::{certs, pkcs8_private_keys};

pub(crate) mod message_storage {
	include!(concat!(env!("OUT_DIR"), "/messagestorage.rs"));
}

mod db;
mod tonic_responder;
mod network;
mod web_service;

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "Dione Server", about="Implementation of the server part of Dione.", version = "0.1.0-alpha")]
struct Opt {
	/// External Address of gRPC server
	///
	/// The address where the gRPC server listens  for incoming connections from clients. Important are interface and port.
	///
	/// Example for listening on all interfaces with port 8010: 0.0.0.0:8010
	#[structopt(long)]
	ex: SocketAddr,

	/// External Address of libp2p part
	///
	/// The address where the libp2p part listens for incoming connections from other peers.
	/// Pass the value in Multiaddress format.
	///
	/// Example for listening on all interfaces with assigned port: /ip4/0.0.0.0/tcp/0
	#[structopt(long, short)]
	listen_address: Option<Multiaddr>,

	/// Clear Address of gRPC server
	///
	/// The address where the gRPC server can be contacted. This is very important for assigned domains or if several interfaces where specified.
	/// Protocol identifier has to be passed along, as well as port. Clear Address and External Address have to be linked.
	///
	/// Example for listening on localhost's port 8010: http://localhost:8010
	#[structopt(long, short)]
	clear_address: Option<String>,

	/// Path to/for Database
	///
	/// Path to the database. If there is no database at the specified location a new one will be created. Target is a directory not a file.
	#[structopt(short = "db", long)]
	db_path: PathBuf,

	/// HTTP port for web server
	///
	/// Specifies the port were the web server belonging to the node will be listening for http requests.
	#[structopt(long, default_value = "8080")]
	web_http_port: usize,

	/// HTTPS port for web server
	///
	/// Specifies the port were the web server belonging to the node will be listening for https requests. Only comes into effect if TLS is configured.
	#[structopt(long, default_value = "8443")]
	web_https_port: usize,

	#[structopt(subcommand)]
	tls: Option<Tls>,
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(about="Subcommand for configuring/enabling/disabling TLS (currently only for webserver)")]
enum Tls {
	/// Disables TLS (default)
	NoTls,
	/// Enables TLS
	Tls {
		/// Path to private key of certificate
		#[structopt(long)]
		private_key: PathBuf,
		/// Path to keychain of certificate
		#[structopt(long)]
		public_key: PathBuf
	}
}

impl Default for Tls {
	fn default() -> Self {
		Self::NoTls
	}
}

fn main() -> anyhow::Result<()> {
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

	let config: Option<ServerConfig> = match opt.tls.clone() {
		None => None,
		Some(e) => {
			match e {
				Tls::NoTls => {
					None
				}
				Tls::Tls { public_key, private_key} => {
					let mut config = ServerConfig::new(NoClientAuth::new());
					let cert_file = &mut BufReader::new(File::open(public_key).unwrap());
					let key_file = &mut BufReader::new(File::open(private_key).unwrap());
					let cert_chain = certs(cert_file).unwrap();
					let mut keys = pkcs8_private_keys(key_file).unwrap();
					config.set_single_cert(cert_chain, keys.remove(0)).unwrap();
					Some(config)
				}
			}
		}
	};

	let web_http_port = opt.web_http_port;
	let web_https_port = opt.web_https_port;

	let _ = System::new();
	let arbiter = Arbiter::new();
	arbiter.spawn(async move {
		web_service::make_server(client_clone, config, web_http_port, web_https_port).await.unwrap();
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

	let mut basic_server = Server::builder();

	let svc = crate::message_storage::message_storage_server::MessageStorageServer::new(greeter);
	let loc = crate::message_storage::location_server::LocationServer::new(locer);
	let server_handler = rt.spawn(async move {
		basic_server
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