pub mod message_storage {
    tonic::include_proto!("messagestorage");
}

use dione_lib::hashing::{cryptographic, non_cryptographic};
use message_storage::message_storage_client::MessageStorageClient;
use message_storage::*;

pub fn verify_hash(hash_type_number: i32, content: &[u8], hash: &[u8]) -> bool {
    let hash_type: HashType = message_storage::HashType::from_i32(hash_type_number).unwrap();
    let hash_algo = match hash_type {
        HashType::Sha512 => cryptographic::sha512_hash_bytes,
        HashType::Adler32 => non_cryptographic::adler_hash_bytes,
        HashType::Lz4 => non_cryptographic::seahash_hash_bytes,
    };
    let compare_hash = hash_algo(content);
    hash == compare_hash
}

pub async fn send_message(net_addr: &'static str, addr: &[u8]) -> bool {
    let mut client = MessageStorageClient::connect(net_addr)
        .await
        .unwrap();

    let content = b"hdheh";

    let request = tonic::Request::new(SaveMessageRequest {
        addr: addr.to_vec(),
        content: content.to_vec(),
    });


    let response = match client.save_message(request).await {
        Ok(d) => d,
        Err(_) => { return false },
    };

    println!("Response = {:?}", response);

    let response_inner = response.into_inner();
    let hash_type = response_inner.hash_type.unwrap();
    let ex_hash = response_inner.hash.unwrap();

    verify_hash(hash_type, b"hdheh", &ex_hash)
}

pub async fn get_message(net_addr: &'static str, addr: &[u8]) -> bool {
    let mut client = MessageStorageClient::connect(net_addr)
        .await
        .unwrap();

    let request = tonic::Request::new(GetMessageRequest {
        addr: addr.to_vec(),
    });

    /*
    let response = match client.get_message(request).await {
        Ok(d) => d,
        Err(_) => { return false }
    };

     */

    let response = client.get_message(request).await.unwrap();

    let response = response.into_inner();

    let content = response.content;
    let addr_m = response.addr;

    addr.to_vec() == addr_m && content == b"hdheh".to_vec()
}

#[tokio::test]
async fn send_message_test() {
    let res = send_message("http://localhost:50051", b"Alice")
        .await;
    assert!(res)
}


#[tokio::test]
async fn send_get_message_test() {
    let res1 = send_message("http://localhost:50051", b"Bob")
        .await;
    let res2 = get_message("http://localhost:50051", b"Bob")
        .await;
    assert!(res2)
}