use crate::tonic_responder::save_message::MessageStorer;
use tonic::transport::Server;
use tracing::Level;

pub(crate) mod message_storage {
    tonic::include_proto!("messagestorage");
}

mod tonic_responder;

#[tokio::main]
async fn main() {
    let collector = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(collector)
        .expect("Something fucked up during setting up collector");

    let addr = "[::1]:50051".parse().unwrap();
    let greeter = MessageStorer::default();

    println!("Storer listening on {}", addr);

    let svc = crate::message_storage::message_storage_server::MessageStorageServer::new(greeter);

    Server::builder()
        .add_service(svc)
        .serve(addr)
        .await
        .unwrap();
}
