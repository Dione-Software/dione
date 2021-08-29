use crate::message_storage::ServerAddressType;
use sled::Db;
use std::collections::HashSet;
use crate::KNOWN_HOSTS_KEY;
use serde::{Serialize, Deserialize};
use tokio::runtime::Runtime;
use crate::net::{get_server_for_address, NetError, get_server_for_message};
use rand::rngs::OsRng;
use rand::prelude::*;
use thiserror::Error;

#[cfg(test)]
use sled::open;

#[derive(Error, Debug)]
pub enum HostError {
	#[error("No hosts in storage")]
	NoHosts,
	#[error("Error in network layer")]
	NetError {
		#[from]
		source: super::net::NetError,
	},
	#[error("No Server for message found")]
	NoServerForMessage,
}

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone)]
pub struct Host {
	pub kind: ServerAddressType,
	pub address: String,
}

impl Host {
	pub fn from_net_prop(kind: &ServerAddressType, inp: &str) -> Self {
		Self {
			kind: kind.clone(),
			address: inp.to_string(),
		}
	}
}

pub struct KnownHosts {
	db: Db,
	hosts: HashSet<Host>
}

impl KnownHosts {
	pub fn new(db: Db) -> Self {
		let hosts = Default::default();
		let hosts_bytes = bincode::serialize(&hosts).unwrap();
		let _ = db.insert(KNOWN_HOSTS_KEY, hosts_bytes).unwrap();
		Self {
			db,
			hosts,
		}
	}

	pub fn add(&mut self, host: Host) -> anyhow::Result<()> {
		let current: Vec<u8> = self.db.get(KNOWN_HOSTS_KEY)?.unwrap().to_vec();
		let mut current: HashSet<Host> = bincode::deserialize(&current)?;
		current.insert(host);
		let current_bytes = bincode::serialize(&current)?;
		self.hosts = current;
		let _ = self.db.insert(KNOWN_HOSTS_KEY, current_bytes)?;
		Ok(())
	}

	pub fn remove(&mut self, host: Host) -> anyhow::Result<()> {
		let current: Vec<u8> = self.db.get(KNOWN_HOSTS_KEY)?.unwrap().to_vec();
		let mut current: HashSet<Host> = bincode::deserialize(&current)?;
		current.remove(&host);
		let current_bytes = bincode::serialize(&current)?;
		self.hosts = current;
		let _ = self.db.insert(KNOWN_HOSTS_KEY, current_bytes)?;
		Ok(())
	}

	pub fn get_server_for_address(&mut self, rt: &Runtime, message_address: &[u8]) -> Result<(ServerAddressType, String), HostError> {
		loop {
			let server = match self.hosts.iter().choose(&mut OsRng) {
				Some(d) => d.to_owned(),
				None => {
					return Err(HostError::NoHosts);
				}
			};
			break match get_server_for_address(rt, server.address.clone(), message_address) {
				Ok(d) => Ok(d),
				Err(e) => {
					match e {
						NetError::TonicError { .. } => {
							self.remove(server.clone()).unwrap();
							continue;
						}
						NetError::ServerResponseErr(e) => {
							let err = HostError::NetError {source: NetError::ServerResponseErr(e)};
							return Err(err)
						}
					}
				}
			}
		}
	}

	pub fn get_server_for_message(&mut self, rt: &Runtime, message_address: &[u8]) -> Result<(ServerAddressType, String), HostError> {
		loop {
			let server = match self.hosts.iter().choose(&mut OsRng) {
				Some(d) => d.to_owned(),
				None => {
					return Err(HostError::NoHosts);
				}
			};
			break match get_server_for_message(rt, server.address.clone(), message_address) {
				Ok(d) => Ok(match d.get(0) {
					Some(d) => d.to_owned(),
					None => {
						return Err(HostError::NoServerForMessage)
					}
				}),
				Err(e) => {
					match e {
						NetError::TonicError { .. } => {
							self.remove(server.clone()).unwrap();
							continue;
						}
						NetError::ServerResponseErr(e) => {
							let err = HostError::NetError {source: NetError::ServerResponseErr(e)};
							return Err(err)
						}
					}
				}
			}
		}
	}
}

#[test]
#[ignore]
fn get_server_for_address_test() {
	let db = open("get_server_for_address_test_db").unwrap();
	let mut known_hosts = KnownHosts::new(db);

	let host_1 = Host::from_net_prop(&ServerAddressType::Clear, "http://127.0.0.1:8010");
	let host_2 = Host::from_net_prop(&ServerAddressType::Clear, "http://127.0.0.1:8000");

	known_hosts.add(host_1).unwrap();
	known_hosts.add(host_2).unwrap();

	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap();

	let message_address = b"binarydata";

	let _ = known_hosts.get_server_for_address(&rt, message_address).unwrap();
}