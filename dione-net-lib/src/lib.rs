mod net;

pub mod message_storage {
    tonic::include_proto!("messagestorage");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
