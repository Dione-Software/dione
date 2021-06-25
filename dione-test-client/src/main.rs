use message_storage::*;
use message_storage::HashType;
use message_storage::message_storage_client::MessageStorageClient;
use dione_lib::hashing::*;

pub mod message_storage {
	tonic::include_proto!("messagestorage");
}

fn verify_hash(hash_type_number: i32, content: &[u8], hash: &[u8]) -> bool {
	let hash_type: HashType = message_storage::HashType::from_i32(hash_type_number).unwrap();
	let hash_algo = match hash_type {
		HashType::Sha512 =>  { cryptographic::sha512_hash_bytes },
		HashType::Adler32 => { non_cryptographic::adler_hash_bytes },
		HashType::Lz4 => { non_cryptographic::seahash_hash_bytes },
	};
	let compare_hash = hash_algo(content);
	hash == compare_hash
}

#[tokio::main]
async fn main() {

	let mut client = MessageStorageClient::connect("http://127.0.0.1:50051")
		.await
		.unwrap();

	let content = b"hdheh";

	let request = tonic::Request::new(SaveMessageRequest {
		addr: b"Felix".to_vec(),
		content: content.to_vec(),
	});

	let response = client.save_message(request).await.unwrap();

	println!("Response = {:?}", response);

	let response_inner = response.into_inner();
	let hash_type = response_inner.hash_type.unwrap();
	let ex_hash = response_inner.hash.unwrap();

	let result_comparison = verify_hash(hash_type, b"hdheh", &ex_hash);
	println!("Result of comparison => {}", result_comparison);
}
