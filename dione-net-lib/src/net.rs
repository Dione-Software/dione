use crate::message_storage::location_client::LocationClient;
use crate::message_storage::message_storage_client::MessageStorageClient;
use crate::message_storage::ServerLocRequest;
use crate::message_storage::{SaveMessageRequest, SaveMessageResponse, GetMessageResponse, GetMessageRequest};
use crate::message_storage::ServerAddressType;
use tonic::Request;
use tokio::runtime::Runtime;
use std::net::Shutdown::Read;

impl From<i32> for ServerAddressType {
	fn from(number: i32) -> Self {
		match number {
			1 => Self::Onion,
			2 => Self::Clear,
			3 => Self::Ipv4,
			4 => Self::Ipv6,
			_ => {
				panic!("This shouldn't have happened")
			}
		}
	}
}


pub fn get_server_for_address(rt: Runtime, server_address: String, message_address: &[u8]) -> anyhow::Result<(ServerAddressType, String)> {
	let mut client = rt.block_on(LocationClient::connect(server_address))?;

	let request = Request::new(ServerLocRequest {
		addr: message_address.to_vec(),
	});

	let response = rt.block_on(client.look_up(request))?;
	let response = response.into_inner();
	let addr_type = ServerAddressType::from(response.addrtype);
	let addr = response.addr;
	Ok((addr_type, addr))
}

pub fn get_server_for_message(rt: Runtime, server_address: String, message_address: &[u8]) -> anyhow::Result<Vec<(ServerAddressType, String)>> {
	let mut client = rt.block_on(LocationClient::connect(server_address))?;

	let request = Request::new(ServerLocRequest {
		addr: message_address.to_vec(),
	});

	let response = rt.block_on(client.message_look_up(request))?;
	let response = response.into_inner();

	let addresses = response.addrs.iter().map(|e| (ServerAddressType::from(e.addrtype), e.clone().addr)).collect();
	Ok(addresses)
}

pub fn save_message(rt: Runtime, server_address: String, message_address: &[u8], content: &[u8]) -> anyhow::Result<SaveMessageResponse> {
	let mut client = rt.block_on(MessageStorageClient::connect(server_address))?;

	let request = Request::new(SaveMessageRequest {
		addr: message_address.to_vec(),
		content: content.to_vec(),
	});

	let response = rt.block_on(client.save_message(request))?;
	let response = response.into_inner();

	Ok(response)
}

pub fn get_message(rt: Runtime, server_address: String, message_address: &[u8]) -> anyhow::Result<GetMessageResponse> {
	let mut client = rt.block_on(MessageStorageClient::connect(server_address))?;

	let request = Request::new(GetMessageRequest {
		addr: message_address.to_vec(),
	});

	let response = rt.block_on(client.get_message(request))?;
	let response = response.into_inner();

	Ok(response)
}