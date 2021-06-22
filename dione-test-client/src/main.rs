pub mod message_storage {
    tonic::include_proto!("messagestorage");
}

use message_storage::message_storage_client::MessageStorageClient;
use message_storage::*;

#[tokio::main]
async fn main() {
    let mut client = MessageStorageClient::connect("http://[::1]:50051")
        .await
        .unwrap();

    let request = tonic::Request::new(SaveMessageRequest {
        addr: b"Felix".to_vec(),
        content: b"hdheh".to_vec(),
    });

    let response = client.save_message(request).await.unwrap();

    println!("Response = {:?}", response);
}
