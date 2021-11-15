use tracing::*;

use libp2p::{NetworkBehaviour, Multiaddr, PeerId, Swarm};
use libp2p::kad::{Kademlia, KademliaEvent, QueryId, QueryResult, GetProvidersOk, GetRecordOk, Quorum, Record, GetClosestPeersOk, PutRecordOk, AddProviderOk};
use libp2p::kad::store::MemoryStore;
use std::error::Error;
use tokio::sync::{oneshot, mpsc};
use std::collections::{HashSet, HashMap};
use libp2p::swarm::{SwarmEvent, SwarmBuilder, DialError};
use libp2p::multiaddr::Protocol;
use libp2p::kad::record::Key;
use tokio_stream::StreamExt;
use serde::{Serialize, Deserialize};

type ShareAddress = Vec<u8>;

pub async fn new() -> Result<(Client, EventLoop), Box<dyn Error>> {
	let id_keys = libp2p::identity::Keypair::generate_ed25519();
	let peer_id = id_keys.public().to_peer_id();

	let transport = libp2p::development_transport(id_keys).await.unwrap();

	let swarm = SwarmBuilder::new(
		transport,
		ComposedBehaviour {
			kademlia: Kademlia::new(peer_id, MemoryStore::new(peer_id)),
		},
		peer_id
	).build();

	let (command_sender, command_receiver) = mpsc::channel(1);

	Ok((
		Client {
			sender: command_sender,
		},
		EventLoop::new(swarm, command_receiver)
		))
}

#[derive(Clone, Debug)]
pub struct Client {
	sender: mpsc::Sender<Command>,
}

impl Client {
	#[instrument]
	pub async fn start_listening(
		&mut self,
		addr: Multiaddr,
	) -> anyhow::Result<()> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Command::StartListening {addr, sender })
			.await
			.expect("Command reciever not to be dropped");
		receiver.await.expect("Sender not to be dropped")
	}

	#[instrument]
	pub async fn dial(
		&mut self,
		peer_id: PeerId,
		peer_addr: Multiaddr,
	) -> anyhow::Result<()> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Command::Dial {
				peer_id,
				peer_addr,
				sender
			})
			.await
			.expect("Command receiver not to be dropped");
		receiver.await.expect("Sender not to be dropped")
	}

	#[instrument]
	pub async fn start_providing(&self, share_addr: ShareAddress) {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Command::StartProviding { share_addr, sender })
			.await
			.expect("Command receiver not to be dropped.");
		receiver.await.expect("Sender not to be dropped.")
	}

	#[instrument]
	pub async fn stop_providing(&self, share_addr: ShareAddress) {
		self.sender
			.send(Command::StopProviding { share_addr })
			.await
			.expect("Receiver not to be dropped");
	}

	#[instrument]
	pub async fn get_providers(&self, share_addr: ShareAddress) -> HashSet<PeerId> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Command::GetProviders { share_addr, sender })
			.await
			.expect("Command receiver not to be dropped.");
		receiver.await.expect("Sender not to be dropped.")
	}

	#[instrument]
	pub async fn get_clear_addr(&self, peer_id: PeerId) -> anyhow::Result<ServerAddrBundle> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Command::GetClearAddr { peer_id, sender })
			.await
			.expect("Command receiver not to be dropped.");
		receiver.await.expect("Sender not to be dropped.")
	}

	#[instrument]
	pub async fn put_clear_addr(&self, addr_type: crate::message_storage::ServerAddressType, addr: String) {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Command::PutClearAddr { addr_type, addr, sender })
			.await
			.expect("Command receiver not to be dropped.");
		println!("Waiting for clear addr");
		receiver.await.expect("Sender not to be dropped")
	}

	#[instrument]
	pub async fn get_closest_peer(&self, addr: Vec<u8>) -> anyhow::Result<PeerId> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Command::GetClosestPeer { addr, sender })
			.await
			.expect("Command receiver not to be dropped.");
		receiver.await.expect("Sender not to be dropped")
	}

	#[instrument]
	pub async fn get_listen_address(&self) -> anyhow::Result<Vec<Multiaddr>> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Command::GetListenAddress { sender })
			.await
			.expect("Command receiver not to be dropped.");
		Ok(receiver.await.expect("Error thrown in lookup").expect("Sender not to be dropped"))
	}
}

#[derive(NetworkBehaviour)]
#[behaviour(event_process = false, out_event = "ComposedEvent")]
struct ComposedBehaviour {
	kademlia: Kademlia<MemoryStore>,
}

#[derive(Debug)]
enum ComposedEvent {
	Kademlia(KademliaEvent),
}

impl From<KademliaEvent> for ComposedEvent {
	fn from(event: KademliaEvent) -> Self {
		ComposedEvent::Kademlia(event)
	}
}

#[derive(Debug)]
enum Command {
	StartListening {
		addr: Multiaddr,
		sender: oneshot::Sender<anyhow::Result<()>>,
	},
	Dial {
		peer_id: PeerId,
		peer_addr: Multiaddr,
		sender: oneshot::Sender<anyhow::Result<()>>,
	},
	StartProviding {
		share_addr: ShareAddress,
		sender: oneshot::Sender<()>,
	},
	StopProviding {
		share_addr: ShareAddress,
	},
	GetProviders {
		share_addr: ShareAddress,
		sender: oneshot::Sender<HashSet<PeerId>>,
	},
	GetClearAddr {
		peer_id: PeerId,
		sender: oneshot::Sender<anyhow::Result<ServerAddrBundle>>,
	},
	PutClearAddr {
		addr_type: crate::message_storage::ServerAddressType,
		addr: String,
		sender: oneshot::Sender<()>,
	},
	GetClosestPeer {
		addr: ShareAddress,
		sender: oneshot::Sender<anyhow::Result<PeerId>>,
	},
	GetListenAddress {
		sender: oneshot::Sender<anyhow::Result<Vec<Multiaddr>>>,
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerAddrBundle {
	peer_id: Vec<u8>,
	pub addr: String,
	pub addr_type: crate::message_storage::ServerAddressType,
}


pub struct EventLoop {
	swarm: Swarm<ComposedBehaviour>,
	command_receiver: mpsc::Receiver<Command>,
	pending_dial: HashMap<PeerId, oneshot::Sender<anyhow::Result<()>>>,
	pending_start_providing: HashMap<QueryId, oneshot::Sender<()>>,
	pending_get_providers: HashMap<QueryId, oneshot::Sender<HashSet<PeerId>>>,
	pending_put_clear_addr: HashMap<QueryId, oneshot::Sender<()>>,
	pending_get_clear_addr: HashMap<QueryId, oneshot::Sender<anyhow::Result<ServerAddrBundle>>>,
	pending_get_closest_peer: HashMap<QueryId, oneshot::Sender<anyhow::Result<PeerId>>>,
	providing: HashSet<Key>,
}

impl EventLoop {
	fn new(
		swarm: Swarm<ComposedBehaviour>,
		command_receiver: mpsc::Receiver<Command>,
	) -> Self {
		Self {
			swarm,
			command_receiver,
			pending_dial: Default::default(),
			pending_start_providing: Default::default(),
			pending_get_providers: Default::default(),
			pending_get_clear_addr: Default::default(),
			pending_put_clear_addr: Default::default(),
			pending_get_closest_peer: Default::default(),
			providing: Default::default()
		}
	}

	pub async fn run(&mut self) {
		loop {
			tokio::select! {
				event = self.swarm.next() => {
					self.handle_event(event.unwrap()).await
				},
				command = self.command_receiver.recv() => match command {
					Some(c) => self.handle_command(c).await,
					None => return,
				}
			}
		}
	}

	#[instrument(skip(self))]
	async fn handle_event(
		&mut self,
		event: SwarmEvent<
			ComposedEvent,
			std::io::Error
		>
	) {
		match event {
			SwarmEvent::Behaviour(ComposedEvent::Kademlia(
			KademliaEvent::OutboundQueryCompleted {
				id,
				result: QueryResult::StartProviding(Ok(AddProviderOk{ key } )),
				..
			}, )) => {
				self.providing.insert(key);
				let sender: oneshot::Sender<()> = self
					.pending_start_providing
					.remove(&id)
					.expect("Completed query to be previously pending");
				let _ = sender.send(());
			}
			SwarmEvent::Behaviour(ComposedEvent::Kademlia(
			KademliaEvent::OutboundQueryCompleted {
				id,
				result: QueryResult::GetProviders(Ok(GetProvidersOk{ providers, key, .. })),
				..
			})) => {
				let mut providers = providers;
				if self.providing.contains(&key) {
					providers.insert(*self.swarm.local_peer_id());
				}
				let _ = self
					.pending_get_providers
					.remove(&id)
					.expect("Completed query to be previously pending")
					.send(providers);
			}
			SwarmEvent::Behaviour(ComposedEvent::Kademlia(
			KademliaEvent::OutboundQueryCompleted {
				id,
				result: QueryResult::GetRecord(Ok(GetRecordOk{ records, .. })), ..
			})) => {
				let bundle = match records.get(0) {
					None => {
						Err(anyhow::Error::msg("No Clear address record returned from query"))
					}
					Some(d) => {
						let data = d.record.value.clone();
						let bundle: anyhow::Result<ServerAddrBundle, bincode::Error> = bincode::deserialize(&data);
						match bundle {
							Ok(r) => Ok(r),
							Err(e) => Err(anyhow::Error::from(e)),
						}
					}
				};
				let _ = self
					.pending_get_clear_addr
					.remove(&id)
					.expect("Completed query to previously pending")
					.send(bundle);
			}
			SwarmEvent::Behaviour(ComposedEvent::Kademlia(
			KademliaEvent::OutboundQueryCompleted {
				id,
				result: QueryResult::GetClosestPeers(Ok(GetClosestPeersOk { peers, key })), ..
			})) => {
				let key = libp2p::kad::kbucket::Key::from(key);
				let host_peer_id = *self.swarm.local_peer_id();
				let host_peer_key = libp2p::kad::kbucket::Key::from(host_peer_id);
				let host_distance = host_peer_key.distance(&key);
				println!("Closest Peers => {:?}", peers);
				let mut peer_id = peers.get(0).unwrap().to_owned();
				let remote_peer_key = libp2p::kad::kbucket::Key::from(peer_id);
				let remote_distance = remote_peer_key.distance(&key);
				if remote_distance > host_distance {
					peer_id = host_peer_id;
				}
				println!("Returning peer => {:?}", peer_id);
				let _ = self
					.pending_get_closest_peer
					.remove(&id)
					.expect("Completed query to previously pending")
					.send(Ok(peer_id));
			},
			SwarmEvent::Behaviour(ComposedEvent::Kademlia(
			KademliaEvent::OutboundQueryCompleted {
				id,
				result: QueryResult::PutRecord(Ok(PutRecordOk{ .. })), ..
			})) => {
				let _ = self.pending_put_clear_addr
					.remove(&id)
					.expect("Completed query to previously pending")
					.send(());
			},
			SwarmEvent::Behaviour(ComposedEvent::Kademlia( .. )) => {}
			SwarmEvent::NewListenAddr { address, .. } => {
				let local_peer_id = *self.swarm.local_peer_id();
				println!("Local node is listening on {:?}",
					address.with(Protocol::P2p(local_peer_id.into()))
				)
			}
			SwarmEvent::IncomingConnection { local_addr, send_back_addr } => {
				tracing::debug!("local addr {:?}", local_addr);
				tracing::debug!("send back addr {:?}", send_back_addr);
				let mut remote_addr = send_back_addr.clone();
				let remote_port = remote_addr.pop().unwrap();
				let new_remote_port = match remote_port {
					Protocol::Tcp(p) => {
						Protocol::Tcp(p - 1)
					},
					_ => {
						tracing::error!("This is a fatal error this shouldn't have happened");
						remote_port
					}
				};
				remote_addr.push(new_remote_port);
				self.swarm.dial_addr(remote_addr).expect("Error dialing send back addr");
			},
			SwarmEvent::ConnectionEstablished {
				peer_id, endpoint, ..
			} => {
				println!("Adding peer {} with address {}", peer_id, endpoint.get_remote_address().clone());
				self.swarm.behaviour_mut().kademlia.add_address(&peer_id, endpoint.get_remote_address().clone());
				if endpoint.is_dialer() {
					if let Some(sender) = self.pending_dial.remove(&peer_id) {
						let _ = sender.send(Ok(()));
					}
				}
			}
			SwarmEvent::OutgoingConnectionError {
				error,
				peer_id
			} => {
				tracing::error!("Had outgoing connection error {:?}", &error);
				/*
				match peer_id {
					Some(d) => {
					if let Some(sender) = self.pending_dial.remove(&d) {
						let _ = sender.send(Err(anyhow::Error::from(error)));
					}},
					None => {}
				}
				 */

			}

			SwarmEvent::ConnectionClosed { .. } => {},
			SwarmEvent::Dialing( .. ) => {},
			e => panic!("{:?}", e),
		}
	}

	#[instrument(skip(self))]
	async fn handle_command(&mut self, command: Command) {
		match command {
			Command::StartListening { addr, sender } => {
				let _ = match self.swarm.listen_on(addr) {
					Ok(_) => sender.send(Ok(())),
					Err(e) => sender.send(Err(anyhow::Error::from(e))),
				};
			}
			Command::Dial { peer_id, peer_addr, sender } => {
				if let std::collections::hash_map::Entry::Vacant(_) = self.pending_dial.entry(peer_id) {

					self.swarm
						.behaviour_mut()
						.kademlia
						.add_address(&peer_id, peer_addr.clone());
					match self
						.swarm
						.dial_addr(peer_addr.with(Protocol::P2p(peer_id.into())))
					{
						Ok(()) => {
							println!("Dialing");
							self.pending_dial.insert(peer_id, sender);
						}
						Err(e) => {
							let _ = sender.send(Err(anyhow::Error::from(e)));
						}
					}
				}
			}
			Command::StartProviding { share_addr, sender } => {
				let key: Key = share_addr.to_vec().into();
				let query_id = self
					.swarm
					.behaviour_mut()
					.kademlia
					.start_providing(key)
					.expect("No store error.");
				self.pending_start_providing.insert(query_id, sender	);
			}
			Command::StopProviding { share_addr } => {
				let key: Key = share_addr.to_vec().into();
				self
					.swarm
					.behaviour_mut()
					.kademlia
					.stop_providing(&key);
				self.providing.remove(&key);
			}
			Command::GetProviders { share_addr, sender } => {
				let key: Key = share_addr.to_vec().into();
				let query_id = self
					.swarm
					.behaviour_mut()
					.kademlia
					.get_providers(key);
				self.pending_get_providers.insert(query_id, sender);
			}
			Command::GetClearAddr { peer_id, sender } => {
				let query_id = self
					.swarm
					.behaviour_mut()
					.kademlia
					.get_record(&Key::from(peer_id.to_bytes()), Quorum::One);
				self.pending_get_clear_addr.insert(query_id, sender);
			}
			Command::PutClearAddr {addr_type, addr, sender} => {
				let peer_id_bytes = self.swarm.local_peer_id().to_bytes();
				let bundle = ServerAddrBundle {
					peer_id: peer_id_bytes.clone(),
					addr,
					addr_type,
				};
				let data = bincode::serialize(&bundle).unwrap();
				let _ = self.swarm.behaviour_mut().kademlia.bootstrap();
				let query_id = self
					.swarm
					.behaviour_mut()
					.kademlia
					.put_record(Record::new(Key::from(peer_id_bytes), data), Quorum::Majority).unwrap();
				self.pending_put_clear_addr.insert(query_id, sender);
			}
			Command::GetClosestPeer { addr, sender } => {
				let query_id = self
					.swarm
					.behaviour_mut()
					.kademlia
					.get_closest_peers(addr);
				self.pending_get_closest_peer.insert(query_id, sender);
			}
			Command::GetListenAddress { sender } => {
				let local_peer_id = self.swarm.local_peer_id().to_owned().into();
				let listen_address: Vec<Multiaddr> = self.swarm.listeners().map(|e| e.to_owned().with(Protocol::P2p(local_peer_id))).collect();
				sender.send(Ok(listen_address)).expect("Receiver not to be dropped");
			}
		}
	}
}