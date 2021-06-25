use tonic::{Request, Response, Status};
use tracing::*;

use crate::message_storage::message_storage_server::MessageStorage;
use crate::message_storage::{SaveMessageRequest, SaveMessageResponse};

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

        let (hash_type, hash) = request_data.hash_content();

        event!(Level::DEBUG, "Calculated Hash");

        let reply = SaveMessageResponse {
            code: 200,
            hash: Some(hash),
            hash_type: Some(hash_type.into()),
        };

        event!(Level::DEBUG, "Formulated Response");

        Ok(Response::new(reply))
    }
}
