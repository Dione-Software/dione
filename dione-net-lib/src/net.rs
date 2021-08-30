use crate::message_storage::location_client::LocationClient;
use crate::message_storage::message_storage_client::MessageStorageClient;
use crate::message_storage::ServerLocRequest;
use crate::message_storage::{SaveMessageRequest, SaveMessageResponse, GetMessageResponse, GetMessageRequest};
use crate::message_storage::ServerAddressType;
use tonic::{Request, Status};
use tokio::runtime::Runtime;

#[derive(Debug, thiserror::Error)]
pub enum NetError {
	#[error("Error in tonic transport layer")]
	TonicError {
		#[from]
		source: tonic::transport::Error,
	},
	#[error("Server threw error with responding status => {0:?}")]
	ServerResponseErr(Status)
}

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


pub fn get_server_for_address(rt: &Runtime, server_address: String, message_address: &[u8]) -> Result<(ServerAddressType, String), NetError> {
	let mut client = match rt.block_on(LocationClient::connect(server_address)) {
		Ok(d) => d,
		Err(e) => {
			let err = NetError::TonicError {
				source: e,
			};
			return Err(err)
		}
	};

	let request = Request::new(ServerLocRequest {
		addr: message_address.to_vec(),
	});

	let response = match rt.block_on(client.look_up(request)) {
		Ok(d) => d,
		Err(e) => {
			let err = NetError::ServerResponseErr(e);
			return Err(err)
		}
	};
	let response = response.into_inner();
	let addr_type = ServerAddressType::from(response.addrtype);
	let addr = response.addr;
	Ok((addr_type, addr))
}

pub fn get_server_for_message(rt: &Runtime, server_address: String, message_address: &[u8]) -> Result<Vec<(ServerAddressType, String)>, NetError> {
	let mut client = match rt.block_on(LocationClient::connect(server_address)) {
		Ok(d) => d,
		Err(e) => {
			let err = NetError::TonicError {
				source: e,
			};
			return Err(err)
		}
	};

	let request = Request::new(ServerLocRequest {
		addr: message_address.to_vec(),
	});

	let response = match rt.block_on(client.message_look_up(request)) {
		Ok(d) => d,
		Err(e) => {
			let err = NetError::ServerResponseErr(e);
			return Err(err)
		}
	};
	let response = response.into_inner();

	let addresses = response.addrs.iter().map(|e| (ServerAddressType::from(e.addrtype), e.clone().addr)).collect();
	Ok(addresses)
}

pub fn save_message(rt: &Runtime, server_address: String, message_address: &[u8], content: &[u8]) -> Result<SaveMessageResponse, NetError> {
	let mut client = match rt.block_on(MessageStorageClient::connect(server_address)) {
		Ok(d) => d,
		Err(e) => {
			let err = NetError::TonicError {
				source: e,
			};
			return Err(err)
		}
	};

	let request = Request::new(SaveMessageRequest {
		addr: message_address.to_vec(),
		content: content.to_vec(),
	});

	let response = match rt.block_on(client.save_message(request)) {
		Ok(d) => d,
		Err(e) => {
			let err = NetError::ServerResponseErr(e);
			return Err(err)
		}
	};
	let response = response.into_inner();

	Ok(response)
}

pub fn get_message(rt: &Runtime, server_address: String, message_address: &[u8]) -> Result<GetMessageResponse, NetError> {
	let mut client = match rt.block_on(MessageStorageClient::connect(server_address)) {
		Ok(d) => d,
		Err(e) => {
			let err = NetError::TonicError {
				source: e,
			};
			return Err(err)
		}
	};

	let request = Request::new(GetMessageRequest {
		addr: message_address.to_vec(),
	});

	let response = match rt.block_on(client.get_message(request)) {
		Ok(d) => d,
		Err(e) => {
			let err = NetError::ServerResponseErr(e);
			return Err(err)
		}
	};
	let response = response.into_inner();

	Ok(response)
}