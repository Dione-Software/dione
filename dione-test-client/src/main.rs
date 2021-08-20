use dione_test_client_lib::{send_message, get_message};

pub mod message_storage {
	tonic::include_proto!("messagestorage");
}



#[tokio::main]
async fn main() {
	let res = send_message("http://127.0.0.1:50051", b"nnoonn").await;
	println!("Result => {:?}", res);
	let res = get_message("http://127.0.0.1:50051", b"nnoonn").await;
	println!("Result => {:?}", res);
}
