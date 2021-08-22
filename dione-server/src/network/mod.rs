use tracing::*;

use libp2p::{NetworkBehaviour, Multiaddr, PeerId, Swarm};
use libp2p::mdns::{Mdns, MdnsEvent};
use libp2p::kad::{Kademlia, KademliaEvent, QueryId, QueryResult, GetProvidersOk, GetRecordOk, Quorum, Record, GetClosestPeersOk, PutRecordOk};
use libp2p::kad::store::MemoryStore;
use std::error::Error;
use tokio::sync::{oneshot, mpsc};
use std::collections::{HashSet, HashMap};
use libp2p::swarm::{SwarmEvent, SwarmBuilder};
use libp2p::core::either::EitherError;
use libp2p::multiaddr::Protocol;
use libp2p::kad::record::Key;
use tokio_stream::StreamExt;
use serde::{Serialize, Deserialize};

type ShareAddress = Vec<u8>;

pub async fn new() -> Result<(Client, EventLoop), Box<dyn Error>> {
	let id_keys = libp2p::identity::Keypair::generate_ed25519();
	let peer_id = id_keys.public().into_peer_id();

	let swarm = SwarmBuilder::new(
		libp2p::development_transport(id_keys).await?,
		ComposedBehaviour {
			mdns: Mdns::new(Default::default()).await?,
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
	) -> Result<(), Box<dyn Error + Send>> {
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
	) -> Result<(), Box<dyn Error + Send>> {
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
	pub async fn get_providers(&self, share_addr: ShareAddress) -> HashSet<PeerId> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Command::GetProviders { share_addr, sender })
			.await
			.expect("Command receiver not to be dropped.");
		receiver.await.expect("Sender not to be dropped.")
	}

	#[instrument]
	pub async fn get_clear_addr(&self, peer_id: PeerId) -> Result<ServerAddrBundle, Box<bincode::ErrorKind>> {
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
	pub async fn get_closest_peer(&self, addr: Vec<u8>) -> Result<PeerId, Box<dyn Error + Send>> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Command::GetClosestPeer { addr, sender })
			.await
			.expect("Command receiver not to be dropped.");
		receiver.await.expect("Sender not to be dropped")
	}
}

#[derive(NetworkBehaviour)]
#[behaviour(event_process = false, out_event = "ComposedEvent")]
struct ComposedBehaviour {
	mdns: Mdns,
	kademlia: Kademlia<MemoryStore>,
}

#[derive(Debug)]
enum ComposedEvent {
	Mdns(MdnsEvent),
	Kademlia(KademliaEvent),
}

impl From<MdnsEvent> for ComposedEvent {
	fn from(event: MdnsEvent) -> Self {
		ComposedEvent::Mdns(event)
	}
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
		sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
	},
	Dial {
		peer_id: PeerId,
		peer_addr: Multiaddr,
		sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
	},
	StartProviding {
		share_addr: ShareAddress,
		sender: oneshot::Sender<()>,
	},
	GetProviders {
		share_addr: ShareAddress,
		sender: oneshot::Sender<HashSet<PeerId>>,
	},
	GetClearAddr {
		peer_id: PeerId,
		sender: oneshot::Sender<Result<ServerAddrBundle, Box<bincode::ErrorKind>>>,
	},
	PutClearAddr {
		addr_type: crate::message_storage::ServerAddressType,
		addr: String,
		sender: oneshot::Sender<()>,
	},
	GetClosestPeer {
		addr: ShareAddress,
		sender: oneshot::Sender<Result<PeerId, Box<dyn Error + Send>>>,
	},
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
	pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), Box<dyn Error + Send>>>>,
	pending_start_providing: HashMap<QueryId, oneshot::Sender<()>>,
	pending_get_providers: HashMap<QueryId, oneshot::Sender<HashSet<PeerId>>>,
	pending_put_clear_addr: HashMap<QueryId, oneshot::Sender<()>>,
	pending_get_clear_addr: HashMap<QueryId, oneshot::Sender<Result<ServerAddrBundle, Box<bincode::ErrorKind>>>>,
	pending_get_closest_peer: HashMap<QueryId, oneshot::Sender<Result<PeerId, Box<dyn Error + Send>>>>,
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
			EitherError<void::Void, std::io::Error>
		>
	) {
		match event {
			SwarmEvent::Behaviour(ComposedEvent::Kademlia(
			KademliaEvent::OutboundQueryCompleted {
				id,
				result: QueryResult::StartProviding(_),
				..
			}, )) => {
				let sender: oneshot::Sender<()> = self
					.pending_start_providing
					.remove(&id)
					.expect("Completed query to be previously pending");
				let _ = sender.send(());
			}
			SwarmEvent::Behaviour(ComposedEvent::Kademlia(
			KademliaEvent::OutboundQueryCompleted {
				id,
				result: QueryResult::GetProviders(Ok(GetProvidersOk{ providers, ..})),
				..
			})) => {
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
				let data = records.get(0).unwrap().record.value.clone();
				let bundle: Result<ServerAddrBundle, bincode::Error> = bincode::deserialize(&data);
				let _ = self
					.pending_get_clear_addr
					.remove(&id)
					.expect("Completed query to previously pending")
					.send(bundle);
			}
			SwarmEvent::Behaviour(ComposedEvent::Kademlia(
			KademliaEvent::OutboundQueryCompleted {
				id,
				result: QueryResult::GetClosestPeers(Ok(GetClosestPeersOk { peers, .. })), ..
			})) => {
				println!("Closest Peers => {:?}", peers);
				let peer_id = peers.get(0).unwrap().to_owned();
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
			}
			SwarmEvent::Behaviour(ComposedEvent::Kademlia( .. )) => {}
			SwarmEvent::Behaviour(ComposedEvent::Mdns(MdnsEvent::Discovered(list))) => {
				for (peer_id, multiaddr) in list {
					self.swarm.behaviour_mut().kademlia.add_address(&peer_id, multiaddr);
				}
			}
			SwarmEvent::Behaviour(ComposedEvent::Mdns(MdnsEvent::Expired(list))) => {
				for (peer_id, multiaddr) in list {
					self.swarm.behaviour_mut().kademlia.remove_address(&peer_id, &multiaddr);
				}
			}
			SwarmEvent::NewListenAddr { address, .. } => {
				let local_peer_id = *self.swarm.local_peer_id();
				println!("Local node is listening on {:?}",
					address.with(Protocol::P2p(local_peer_id.into()))
				)
			}
			SwarmEvent::IncomingConnection { .. } => {},
			SwarmEvent::ConnectionEstablished {
				peer_id, endpoint, ..
			} => {
				if endpoint.is_dialer() {
					if let Some(sender) = self.pending_dial.remove(&peer_id) {
						let _ = sender.send(Ok(()));
					}
				}
			}
			SwarmEvent::UnreachableAddr {
				peer_id,
				attempts_remaining,
				error,
				..
			} => {
				if attempts_remaining == 0 {
					if let Some(sender) = self.pending_dial.remove(&peer_id) {
						let _ = sender.send(Err(Box::new(error)));
					}
				}
			},
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
					Err(e) => sender.send(Err(Box::new(e))),
				};
			}
			Command::Dial { peer_id, peer_addr, sender } => {
				if self.pending_dial.contains_key(&peer_id) {
					todo!("Already dialing peer")
				} else {
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
							let _ = sender.send(Err(Box::new(e)));
						}
					}
				}
			}
			Command::StartProviding { share_addr, sender } => {
				let query_id = self
					.swarm
					.behaviour_mut()
					.kademlia
					.start_providing(share_addr.to_vec().into())
					.expect("No store error.");
				self.pending_start_providing.insert(query_id, sender	);
			}
			Command::GetProviders { share_addr, sender } => {
				let query_id = self
					.swarm
					.behaviour_mut()
					.kademlia
					.get_providers(share_addr.to_vec().into());
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
				let query_id = self
					.swarm
					.behaviour_mut()
					.kademlia
					.put_record(Record::new(Key::from(peer_id_bytes), data), Quorum::One).unwrap();
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
		}
	}
}