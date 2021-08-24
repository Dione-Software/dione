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

mod net;
mod session;
mod bundle;
mod user;
mod host;

const HOST_USER_KEY: &[u8] = b"host_user";
const HOST_IDENTITY_KEY_KEY: &[u8] = b"host_identity_key";
const KNOWN_HOSTS_KEY: &[u8] = b"known_hosts";
const NUMBER_SHARES_KEY: &[u8] = b"number_shares";
const HOST_BUNDLE_KEY: &[u8] = b"host_bundle";

pub mod message_storage {
    tonic::include_proto!("messagestorage");
}

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

    pub fn provide_bundle(&mut self) -> anyhow::Result<()> {
        let host_uuid = self.host_user
            .id
            .as_bytes()
            .to_vec();

        let (_, server_address) = self.known_hosts.get_server_for_address(&self.runtime, &host_uuid)?;

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

        let host_bundle = HostBundle::new(self.db.clone(), bundle);
        self.host_bundle = Some(host_bundle);


        let _ = save_message(&self.runtime, server_address, &host_uuid, &bundle_bytes)?;
        Ok(())
    }

    pub fn start_session(&mut self, id: Uuid) -> anyhow::Result<()> {
        let peer_uuid = id.as_bytes().to_vec();

        let (_, server_address) = self.known_hosts.get_server_for_address(&self.runtime, &peer_uuid)?;

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

        let peer_bundle_bytes = get_message(&self.runtime, server_address.clone(), &peer_uuid)?.content;
        let peer_bundle = AliceBundle::from_bytes(&peer_bundle_bytes)?;

        let (kind, magic_ratchet) = host_bundle.init(&peer_bundle)?;

        let host_bundle_bytes = host_bundle.strip().to_bytes()?;

        let mut host_peer_key = self.host_user.id.as_bytes().to_vec();
        let mut seperator = b"-".to_vec();
        host_peer_key.append(&mut seperator);
        host_peer_key.append(&mut peer_uuid.clone());

        let _ = save_message(&self.runtime, server_address, &host_peer_key, &host_bundle_bytes).unwrap();
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use crate::Client;

    #[test]
    fn connect_test() {
        let mut client = Client::new("client1", 3).unwrap();
        let _ = client.connect(String::from("http://127.0.0.1:8000")).unwrap();
    }

    #[test]
    fn provide_test() {
        let mut client = Client::new("provide_test_client", 3).unwrap();
        client.connect(String::from("http://127.0.0.1:8000")).unwrap();
        client.connect(String::from("http://127.0.0.1:8010")).unwrap();
        let _ = client.provide_bundle().unwrap();
        let client1_id = client.host_user.id;

        let mut client2 = Client::new("provide_test_client2", 3).unwrap();
        client2.connect("http://127.0.0.1:8000".to_string()).unwrap();
        client2.connect("http://127.0.0.1:8010".to_string()).unwrap();

        client2.start_session(client1_id).unwrap();
    }
}
