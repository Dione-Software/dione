use std::path::Path;
use async_trait::async_trait;

#[async_trait]
pub trait MessageStoreDb {
	fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> where Self: Sized;
	async fn save_message(&self, address: &[u8], content: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;
	async fn get_message(&self, address: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;
	async fn remove_message(&self, address: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;
	#[cfg(test)]
	fn test_connection<P: AsRef<Path>>(path: P) -> Self;
	#[cfg(test)]
	async fn destroy_test_connection<P: AsRef<Path> + Send>(path: P) -> anyhow::Result<()>;
	fn flush(&self);
}

#[derive(Debug)]
pub struct MessageDb {
	db: sled::Db,
	message_db: sled::Tree,
}

#[async_trait]
impl MessageStoreDb for MessageDb {
	fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
		let db = sled::open(path)?;
		let message_db = db.open_tree("messages")?;
		Ok(MessageDb {
			db,
			message_db
		})
	}

	async fn save_message(&self, address: &[u8], content: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
		match self.message_db.insert(address, content) {
			Ok(d) => Ok(d.map(|d| d.to_vec())),
			Err(e) => Err(anyhow::Error::from(e)),
		}
	}

	async fn get_message(&self, address: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
		let message = self.message_db.get(address)?;
		Ok(message.map(|e| e.to_vec()))
	}

	async fn remove_message(&self, address: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
		let prev_val = self.message_db.remove(address)?;
		Ok(prev_val.map(|e| e.to_vec()))
	}

	#[cfg(test)]
	fn test_connection<P: AsRef<Path>>(path: P) -> Self {
		Self::new(path).expect("Error creating db")
	}

	#[cfg(test)]
	async fn destroy_test_connection<P: AsRef<Path> + Send>(path: P) -> anyhow::Result<()> {
		tokio::fs::remove_dir_all(path).await?;
		Ok(())
	}

	fn flush(&self) {
		self.db.flush().unwrap();
	}
}
