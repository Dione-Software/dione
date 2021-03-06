use tonic::{Request, Response, Status};
use tracing::*;

use crate::message_storage::{SaveMessageRequest, SaveMessageResponse, GetMessageResponse, GetMessageRequest};
use crate::message_storage::message_storage_server::MessageStorage;
use crate::network::Client;

#[cfg(test)]
use crate::network;

use crate::db::MessageStoreDb;

#[cfg(test)]
use crate::db::MessageDb;

use std::fmt::Debug;

#[derive(Debug)]
pub struct MessageStorer<T: MessageStoreDb> {
	db_conn: T,
	client: Client,
}

impl<T: MessageStoreDb> MessageStorer<T> {
	pub(crate) fn new(conn: T, client: Client) -> Self {
		Self {
			db_conn: conn,
			client
		}
	}
}

#[tonic::async_trait]
impl<T: MessageStoreDb + Sync + Send + Debug + 'static> MessageStorage for MessageStorer<T> {
	#[instrument(skip(self, request))]
	async fn save_message(
		&self,
		request: Request<SaveMessageRequest>,
	) -> Result<Response<SaveMessageResponse>, Status> {
		event!(Level::INFO, "Processing Request");

		let client_clone = self.client.clone();

		let request_data = request.into_inner();


		let addr = request_data.addr.clone();

		let dht = tokio::spawn(async move {
			println!("Propagating {:?}", addr);

			event!(Level::DEBUG, "Propagating to DHT");

			client_clone.start_providing(addr).await;

			event!(Level::DEBUG, "Propagated to DHT");
		});

		let (hash_type, hash) = request_data.hash_content();

		event!(Level::DEBUG, "Calculated Hash");

		let reply = SaveMessageResponse {
			code: 200,
			hash: Some(hash),
			hash_type: Some(hash_type.into()),
		};

		event!(Level::DEBUG, "Formulated Response");

		dht.await.unwrap();

		self.db_conn.save_message(&request_data.addr, &request_data.content).await.expect("Error saving message");

		event!(Level::DEBUG, "Saved to DB");


		Ok(Response::new(reply))
	}

	#[instrument(skip(self, request))]
	async fn get_message(&self, request: Request<GetMessageRequest>) -> Result<Response<GetMessageResponse>, Status> {
		println!("Got a request from {:?}", request.remote_addr());
		event!(Level::INFO, "Processing Request");

		let request_data = request.into_inner();

		let content = self.db_conn.remove_message(&request_data.addr)
			.await
			.expect("Didn't get message")
			.expect("Empty");

		let response = GetMessageResponse {
			addr: request_data.addr.clone(),
			content: content.clone()
		};

		self.client.stop_providing(request_data.addr.clone()).await;

		Ok(Response::new(response))
	}
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn save_message() {
	let (client, mut event_loop) = network::new().await.unwrap();

	tokio::spawn(async move {
		event_loop.run().await
	});
	let test_db_path = String::from("test_save_message_net.sled");
	let test_db = MessageDb::test_connection(&test_db_path);
	test_db.flush();
	let message_storer = MessageStorer::new(test_db, client);
	let save_msg_request = Request::new(
		SaveMessageRequest {
			addr: b"thisisatestaddress".to_vec(),
			content: b"This is just testcontent".to_vec()
		}
	);
	let response = message_storer.save_message(save_msg_request)
		.await
		.expect("Error during processing");
	let test_response = Response::new(
		SaveMessageResponse {
			code: 200,
			hash: Some(vec![174, 150, 137, 99, 41, 196, 240, 89, 181, 117, 4, 74, 154, 241, 97, 31, 155, 171, 232, 105, 159, 168, 129, 237, 178, 187, 120, 235, 23, 98, 150, 46, 0, 120, 82, 212, 168, 20, 59, 231, 223, 211, 193, 1, 239, 128, 31, 81, 63, 204, 54, 58, 125, 7, 225, 190, 78, 181, 183, 84, 117, 35, 110, 200]),
			hash_type: Some(1)
		}
	);
	MessageDb::destroy_test_connection(&test_db_path).await.unwrap();
	assert_eq!(test_response.into_inner(), response.into_inner())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn get_message() {
	let (client, mut event_loop) = network::new().await.unwrap();

	tokio::spawn(async move {
		event_loop.run().await
	});
	let test_db_path = String::from("test_get_message_net.sled");
	let test_db = MessageDb::test_connection(&test_db_path);
	test_db.flush();
	let message_storer = MessageStorer::new(test_db, client);
	let save_msg_request = Request::new(
		SaveMessageRequest {
			addr: b"thisisatestaddress".to_vec(),
			content: b"This is just testcontent".to_vec()
		}
	);
	let _ = message_storer.save_message(save_msg_request)
		.await
		.expect("Error during processing");

	let get_msg_request = Request::new(
		GetMessageRequest {
			addr: b"thisisatestaddress".to_vec(),
		}
	);
	let response = message_storer.get_message(get_msg_request)
		.await
		.expect("Error during processing get request")
		.into_inner();
	let test_response = GetMessageResponse {
		addr: b"thisisatestaddress".to_vec(),
		content: b"This is just testcontent".to_vec()
	};
	MessageDb::destroy_test_connection(&test_db_path).await.unwrap();
	assert_eq!(response, test_response)
}