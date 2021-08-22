use crate::message_storage::location_server::Location;
use tonic::{Request, Response, Status, Code};
use crate::message_storage::{ServerLocRequest, ServerLocResponse};
use crate::network::Client;

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
	async fn look_up(&self, request: Request<ServerLocRequest>) -> Result<Response<ServerLocResponse>, Status> {
		let address = request.into_inner().addr;
		println!("Address => {}", String::from_utf8(address.clone()).unwrap());
		let peer_id = match self.client.get_closest_peer(address).await {
			Ok(d) => d,
			Err(e) => {
				let error = e.to_string();
				return Err(Status::new(Code::Internal, error))
			}
		};
		println!("PeerId => {:?}", peer_id);
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
}