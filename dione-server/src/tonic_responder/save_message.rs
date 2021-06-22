use adler::Adler32;
use tonic::{Request, Response, Status};
use tracing::*;

use crate::message_storage::{SaveMessageRequest, SaveMessageResponse};
use crate::message_storage::message_storage_server::MessageStorage;

#[derive(Default, Debug)]
pub struct MessageStorer {}

#[tonic::async_trait]
impl MessageStorage for MessageStorer {
	#[instrument(skip(request))]
	async fn save_message(
		&self,
		request: Request<SaveMessageRequest>,
	) -> Result<Response<SaveMessageResponse>, Status> {
		println!("Got a request from {:?}", request.remote_addr());
		event!(Level::INFO, "Processing Request");

		let request_data = request.into_inner();

		let content = &request_data.content;

		let mut hasher = Adler32::new();

		event!(Level::DEBUG, "Calculating Hash");

		hasher.write_slice(content);
		let checksum = hasher.checksum().to_be_bytes();

		event!(Level::DEBUG, "Calculated Hash");

		let reply = SaveMessageResponse {
			code: 200,
			hash: Some(checksum.to_vec()),
		};

		event!(Level::DEBUG, "Formulated Response");

		Ok(Response::new(reply))
	}
}
