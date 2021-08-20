use crate::message_storage::location_server::Location;
use tonic::{Request, Response, Status};
use crate::message_storage::{ServerLocRequest, ServerLocResponse};

#[derive(Debug, Default)]
pub struct LocationService {}

#[tonic::async_trait]
impl Location for LocationService {
	async fn look_up(&self, request: Request<ServerLocRequest>) -> Result<Response<ServerLocResponse>, Status> {
		todo!()
	}
}