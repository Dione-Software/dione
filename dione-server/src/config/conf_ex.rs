use serde_derive::Deserialize;
use std::str::FromStr;
use std::path::Path;
use std::fs::read_to_string;
use std::net::{SocketAddr, IpAddr};

#[derive(Deserialize, Debug)]
pub(crate) struct NetworkCon {
    ip: String,
    port: u16,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Conf {
    pub node_name: String,
    pub network_con: Services,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Services {
    pub message_storage: NetworkCon,
    pub conf_retrv: Option<NetworkCon>,
}

impl FromStr for Conf {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = Path::new(s);
        let str = read_to_string(path).expect("No Config file");
        println!("String => {}", str);
        toml::from_str(&str)
    }
}

impl From<NetworkCon> for SocketAddr {
    fn from(conf: NetworkCon) -> Self {
        let ip_str = conf.ip;
        let ip = IpAddr::from_str(&ip_str)
            .expect("Error parsing IP-Address");
        SocketAddr::new(ip, conf.port)
    }
}