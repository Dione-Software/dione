use tracing::*;

use crate::message_storage::location_server::Location;
use tonic::{Request, Response, Status, Code};
use crate::message_storage::{ServerLocRequest, ServerLocResponse, MessageLocResponse};
use crate::network::Client;
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
pub struct LocationService {
	client: Client,
}

impl LocationService {
	pub fn new(client: Client) -> Self {
		Self {
			client
		}
	}
}

#[tonic::async_trait]
impl Location for LocationService {
	#[instrument(skip(request))]
	async fn look_up(&self, request: Request<ServerLocRequest>) -> Result<Response<ServerLocResponse>, Status> {
		let address = request.into_inner().addr;

		event!(Level::DEBUG, "Looking for closest peer for address: {:?}", address);

		let peer_id = match self.client.get_closest_peer(address).await {
			Ok(d) => d,
			Err(e) => {
				let error = e.to_string();
				return Err(Status::new(Code::Internal, error))
			}
		};

		event!(Level::DEBUG, "closest peer id: {:?}", peer_id);


		let bundle = match self.client.get_clear_addr(peer_id).await {
			Ok(d) => d,
			Err(e) => {
				let error = e.to_string();
				return Err(Status::new(Code::Internal, error))
			}
		};
		let clear = bundle.addr;
		let addr_type = bundle.addr_type;
		let response = ServerLocResponse {
			addrtype: addr_type.into(),
			addr: clear
		};
		let response = Response::new(response);
		Ok(response)
	}

	#[instrument(skip(request))]
	async fn message_look_up(&self, request: Request<ServerLocRequest>) -> Result<Response<MessageLocResponse>, Status> {
		let address = request.into_inner().addr;

		event!(Level::DEBUG, "Looking for closest peer for address: {:?}", address);

		let peer_ids = self.client.get_providers(address.clone()).await;

		event!(Level::DEBUG, "Found providers: {:?}", peer_ids);

		let mut handle_queue = VecDeque::with_capacity(peer_ids.len());

		for peer_id in peer_ids.clone() {
			let client = self.client.clone();
			let handle = tokio::spawn(async move {
				let res = client.get_clear_addr(peer_id).await.unwrap();

				event!(Level::DEBUG, "Got clear address: {:?} for: {:?}", res, peer_id);

				(peer_id, res)
			});
			handle_queue.push_back(handle);
		}


		let mut id_clear = HashMap::with_capacity(handle_queue.len());

		for e in handle_queue {
				let (id, bundle) = e.await.unwrap();
				id_clear.insert(id, bundle);
		}

		let res = peer_ids
			.iter()
			.map(|e| id_clear.get(e).unwrap())
			.map(|e| ServerLocResponse {
				addrtype: e.addr_type.into(),
				addr: e.addr.clone()
			})
			.collect();

		let response = Response::new(MessageLocResponse {
			addrs: res,
		});

		Ok(response)
	}
}