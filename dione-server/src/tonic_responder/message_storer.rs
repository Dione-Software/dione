use tonic::{Request, Response, Status};
use tracing::*;

use crate::message_storage::{SaveMessageRequest, SaveMessageResponse, GetMessageResponse, GetMessageRequest};
use crate::message_storage::message_storage_server::MessageStorage;
use crate::db::messages_db::MessagesDb;

#[derive(Debug)]
pub struct MessageStorer {
	db_conn: MessagesDb,
}

impl MessageStorer {
	pub(crate) fn new(conn: MessagesDb) -> Self {
		Self {
			db_conn: conn
		}
	}
}

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


		let (hash_type, hash) = request_data.hash_content();

		event!(Level::DEBUG, "Calculated Hash");

		let reply = SaveMessageResponse {
			code: 200,
			hash: Some(hash),
			hash_type: Some(hash_type.into()),
		};

		event!(Level::DEBUG, "Formulated Response");

		self.db_conn.save_message(request_data.content, request_data.addr).await;

		event!(Level::DEBUG, "Saved to DB");

		Ok(Response::new(reply))
	}

	#[instrument(skip(request))]
	async fn get_message(&self, request: Request<GetMessageRequest>) -> Result<Response<GetMessageResponse>, Status> {
		println!("Got a request from {:?}", request.remote_addr());
		event!(Level::INFO, "Processing Request");

		let request_data = request.into_inner();

		let db_response = self.db_conn.get_message(request_data.addr)
			.await
			.expect("Didn't get message");
		let first_message = db_response.get(0).unwrap();

		let response = GetMessageResponse {
			addr: first_message.address.clone(),
			content: first_message.content.clone()
		};

		Ok(Response::new(response))
	}
}
