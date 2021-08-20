use crate::message_storage::location_server::Location;
use tonic::{Request, Response, Status};
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
		todo!()
	}
}