//! Client Library with Networking.
//!
//! This is the easy client interface to Dione. Although it'S currently not, the goal is to be a batteries included library for Dione.
//!
//! The library uses [sled](http://sled.rs) as a storage backend.

use sled::{Db, open};
use std::path::Path;
use crate::user::User;
use dione_lib::cryptography::key_exchange::{IdentityKey, Key};
use crate::host::{Host, KnownHosts};
use tokio::runtime::Runtime;
use crate::net::{get_server_for_address, save_message, get_message};
use std::collections::HashMap;
use uuid::Uuid;
use crate::session::Session;
use crate::bundle::{BundleBuilder, AliceBob, PartnerBundle, BobBundle, AliceBundle, HostBundle};
use dione_lib::cryptography::ratchet::AddressShare;
use serde::{Deserialize, Serialize};

mod net;
pub mod session;
mod bundle;
mod user;
mod host;

const HOST_USER_KEY: &[u8] = b"host_user";
const HOST_IDENTITY_KEY_KEY: &[u8] = b"host_identity_key";
const KNOWN_HOSTS_KEY: &[u8] = b"known_hosts";
const NUMBER_SHARES_KEY: &[u8] = b"number_shares";
const HOST_BUNDLE_KEY: &[u8] = b"host_bundle";

pub(crate) mod message_storage {
    tonic::include_proto!("messagestorage");
}

/// Client for external usage
pub struct Client {
    db: Db,
    runtime: Runtime,
    host_user: User,
    host_identity_key: IdentityKey,
    host_bundle: Option<HostBundle>,
    number_shares: usize,
    known_hosts: KnownHosts,
    sessions: HashMap<Uuid, Session>,
}

impl Client {
    /// Constructs a new client with a Path for the Db and the number of shares this communication uses.
    pub fn new<P: AsRef<Path>>(p: P, number_shares: usize) -> anyhow::Result<Self> {
        let db = open(p)?;

        let host_user = User::default();
        let host_user_bytes = bincode::serialize(&host_user)?;
        let _ = db.insert(HOST_USER_KEY, host_user_bytes)?;

        let host_identity_key = IdentityKey::default();
        let identity_key_bytes = host_identity_key.to_bytes();
        let _ = db.insert(HOST_IDENTITY_KEY_KEY, identity_key_bytes)?;

        let known_hosts = KnownHosts::new(db.clone());

        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

        let number_shares_bytes = number_shares.to_be_bytes().to_vec();
        let _ = db.insert(NUMBER_SHARES_KEY, number_shares_bytes)?;

        let client = Self {
            db,
            host_user,
            host_identity_key,
            host_bundle: None,
            number_shares,
            known_hosts,
            runtime,
            sessions: Default::default(),
        };
        Ok(client)
    }

    /// Initial connection to a server. This is necessary to perform all other functionalities and tests the network connection.
    pub fn connect(&mut self, server_address: String) -> anyhow::Result<()> {
        match get_server_for_address(&self.runtime, server_address.clone(), b"") {
            Ok(d) => {
                let addressed_host = Host::from_net_prop(&message_storage::ServerAddressType::Clear, server_address.as_str());
                self.known_hosts.add(addressed_host)?;
                let ret_host = Host::from_net_prop(&d.0, &d.1);
                self.known_hosts.add(ret_host)?;
            }
            Err(e) => {
                return Err(anyhow::Error::from(e))
            }
        }
        Ok(())
    }

    /// Initial necessary step for establishing a connection to other [Client]. Provides the own message bundle to servers.
    pub fn provide_bundle(&mut self) -> anyhow::Result<()> {
        let host_uuid = self.host_user
            .id
            .as_bytes()
            .to_vec();


        let bundle = BundleBuilder::default()
            .identity_key(self.host_identity_key.clone())
            .number_shares(self.number_shares)
            .partner(AliceBob::Alice)
            .build()?;

        let bundle = match bundle {
            PartnerBundle::Alice(d) => d,
            PartnerBundle::Bob(_) => {
                panic!("The programmer fucked up");
            }
        };

        let bundle_bytes = bundle.strip().to_bytes()?;

        let host_bundle = HostBundle::new(self.db.clone(), *bundle);
        self.host_bundle = Some(host_bundle);


        let (_, server_address) = self.known_hosts.get_server_for_address(&self.runtime, &host_uuid)?;
        let _ = save_message(&self.runtime, server_address, &host_uuid, &bundle_bytes)?;
        Ok(())
    }

    /// Step one for establishing a communication session. Remote peers id is needed.
    pub fn init_one_session(&mut self, id: Uuid) -> anyhow::Result<()> {
        let peer_uuid = id.as_bytes().to_vec();

        let (_, server_address) = self.known_hosts.get_server_for_message(&self.runtime, &peer_uuid)?;
        let peer_bundle_bytes = get_message(&self.runtime, server_address, &peer_uuid)?.content;

        let bundle = BundleBuilder::default()
            .identity_key(self.host_identity_key.clone())
            .number_shares(self.number_shares)
            .partner(AliceBob::Bob)
            .build()?;

        let mut host_bundle = match bundle {
            PartnerBundle::Alice(_) => {
                panic!("The programmer fucked up");
            }
            PartnerBundle::Bob(d) => d,
        };

        let peer_bundle = AliceBundle::from_bytes(&peer_bundle_bytes)?;

        let (kind, magic_ratchet) = host_bundle.init(&peer_bundle)?;

        let host_bundle_bytes = host_bundle.strip().to_bytes()?;

        let session = Session::new(kind, magic_ratchet);


        self.sessions.insert(id, session);

        let mut host_peer_key = self.host_user.id.as_bytes().to_vec();
        let mut seperator = b"-".to_vec();
        host_peer_key.append(&mut seperator);
        host_peer_key.append(&mut peer_uuid.clone());

        let (_, server_address) = self.known_hosts.get_server_for_address(&self.runtime, &host_peer_key)?;
        let _ = save_message(&self.runtime, server_address, &host_peer_key, &host_bundle_bytes).unwrap();
        Ok(())
    }

    /// Step two for establishing a communication session. Remote peers id is needed.
    pub fn init_two_session(&mut self, id: Uuid) -> anyhow::Result<()> {
        let peer_uuid = id.as_bytes().to_vec();
        let mut host_peer_key = peer_uuid;
        let mut seperator = b"-".to_vec();
        host_peer_key.append(&mut seperator);
        host_peer_key.append(&mut self.host_user.id.as_bytes().to_vec());

        let (_, server_address) = self.known_hosts.get_server_for_message(&self.runtime, &host_peer_key)?;

        let host_bundle = &self.host_bundle.as_ref().unwrap().bundle;


        let peer_bundle_bytes = get_message(&self.runtime, server_address.clone(), &host_peer_key)?.content;
        let peer_bundle: BobBundle = BobBundle::from_bytes(&peer_bundle_bytes)?;

        let (kind, magic_ratchet) = host_bundle.init(&peer_bundle)?;

        let mut session = Session::new(kind, magic_ratchet);


        let init_message = session.make_init_message()?;
        let init_message_bytes = bincode::serialize(&init_message)?;

        self.sessions.insert(id, session);

        let mut ender = b"!".to_vec();
        host_peer_key.append(&mut ender);

        let _ = save_message(&self.runtime, server_address, &host_peer_key, &init_message_bytes)?;

        Ok(())
    }

    /// Step three for establishing a communication session. Remote peers id is needed.
    pub fn init_three_session(&mut self, id: Uuid) -> anyhow::Result<()> {
        let mut peer_uuid = id.as_bytes().to_vec();

        let mut host_peer_key = self.host_user.id.as_bytes().to_vec();
        let mut seperator = b"-".to_vec();
        host_peer_key.append(&mut seperator);
        host_peer_key.append(&mut peer_uuid);
        let mut ender = b"!".to_vec();
        host_peer_key.append(&mut ender);

        let (_, server_address) = self.known_hosts.get_server_for_message(&self.runtime, &host_peer_key)?;

        let init_message_bytes = get_message(&self.runtime, server_address, &host_peer_key)?.content;
        let init_message: Vec<AddressShare> = bincode::deserialize(&init_message_bytes)?;

        let session = self.sessions.get_mut(&id).unwrap();
        session.process_init_message(init_message);

        let init_message = session.make_init_message()?;
        let init_message_bytes = bincode::serialize(&init_message)?;

        host_peer_key.append(&mut ender);

        let (_, server_address) = self.known_hosts.get_server_for_address(&self.runtime, &host_peer_key)?;
        let _ = save_message(&self.runtime, server_address, &host_peer_key, &init_message_bytes).unwrap();

        self.provide_bundle()?;

        Ok(())
    }

    /// Step four for establishing a communication session. Remote peers id is needed.
    pub fn start_session(&mut self, id: Uuid) -> anyhow::Result<()> {
        let peer_uuid = id.as_bytes().to_vec();
        let mut host_peer_key = peer_uuid;
        let mut seperator = b"-".to_vec();
        host_peer_key.append(&mut seperator);
        host_peer_key.append(&mut self.host_user.id.as_bytes().to_vec());
        let mut ender = b"!".to_vec();
        host_peer_key.append(&mut ender);
        host_peer_key.append(&mut ender);

        let (_, server_address) = self.known_hosts.get_server_for_message(&self.runtime, &host_peer_key)?;
        let init_message_bytes = get_message(&self.runtime, server_address, &host_peer_key)?.content;
        let init_message = bincode::deserialize(&init_message_bytes)?;

        let session = self.sessions.get_mut(&id).unwrap();
        session.process_init_message(init_message);

        self.provide_bundle()?;

        Ok(())
    }

    /// Send message to Uuid. Established connection is necessary.
    pub fn send_message(&mut self, id: Uuid, content: &[u8]) -> anyhow::Result<()> {
        let session = self.sessions.get_mut(&id).unwrap();
        let address_shares = session.send_message(content)?;
        for address_share in address_shares {
            let address = address_share.0;
            let share = address_share.1;
            let (_, server_address) = self.known_hosts.get_server_for_address(&self.runtime, &address)?;
            let _ = save_message(&self.runtime, server_address, &address, &share)?;
        }
        Ok(())
    }

    /// Receiving message from Uuid. Established connection and send message necessary.
    pub fn recv_message(&mut self, id: Uuid) -> anyhow::Result<Vec<u8>> {
        let session = self.sessions.get_mut(&id).unwrap();
        let addresses = session.next_address()?;
        let mut parts = Vec::new();
        for address in addresses {
            let (_, server_address) = self.known_hosts.get_server_for_message(&self.runtime, &address)?;
            let d = get_message(&self.runtime, server_address, &address)?.content;
            let address_share = (address, d);
            parts.push(address_share);
        }
        let d = session.recv_message(&parts)?;
        Ok(d)
    }

    /// Get clients Uuid.
    pub fn get_uuid(&self) -> Uuid {
        self.host_user.id
    }

    /// Get Peers who wich a connection is established.
    pub fn get_peers(&self) -> Vec<Uuid> {
        self.sessions.keys().map(|e| e.to_owned()).collect()
    }

}

#[derive(Deserialize, Serialize)]
struct InitMessageBundle {
    host_bundle: Vec<u8>,
    init_message: Vec<AddressShare>,
}

#[cfg(test)]
mod tests {
    use crate::Client;

    #[test]
    #[ignore]
    fn connect_test() {
        let mut client = Client::new("client1", 3).unwrap();
        let _ = client.connect(String::from("http://127.0.0.1:8010")).unwrap();
    }

    #[test]
    #[ignore]
    fn general_test() {
        let mut client1 = Client::new("provide_test_client", 3).unwrap();
        client1.connect(String::from("http://127.0.0.1:8010")).unwrap();
        client1.connect(String::from("http://127.0.0.1:8011")).unwrap();
        let _ = client1.provide_bundle().unwrap();
        let client1_id = client1.host_user.id;

        let mut client2 = Client::new("provide_test_client2", 3).unwrap();
        client2.connect("http://127.0.0.1:8010".to_string()).unwrap();
        client2.connect("http://127.0.0.1:8011".to_string()).unwrap();
        client2.provide_bundle().unwrap();

        let client2_id = client2.host_user.id;

        client2.init_one_session(client1_id).unwrap();

        client1.init_two_session(client2_id).unwrap();

        client2.init_three_session(client1_id).unwrap();

        client1.start_session(client2_id).unwrap();

        let message_content = b"Hello World".to_vec();
        client1.send_message(client2_id, &message_content).unwrap();
        client1.send_message(client2_id, &message_content).unwrap();

        let received = client2.recv_message(client1_id).unwrap();
        assert_eq!(received, message_content);
        let received = client2.recv_message(client1_id).unwrap();
        assert_eq!(received, message_content);

        client2.send_message(client1_id, &message_content).unwrap();
        client2.send_message(client1_id, &message_content).unwrap();

        let received = client1.recv_message(client2_id).unwrap();
        assert_eq!(received, message_content);
        let received = client1.recv_message(client2_id).unwrap();
        assert_eq!(received, message_content);
    }
}
