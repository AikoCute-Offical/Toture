use serde::{Deserialize, Serialize};
use crate::{proxy::base::SupportedProtocols};


#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub node_id: u8,
    pub webapi_url: String,
    pub webapi_key: String,
    pub check_interval: u64, 
    pub submit_interval: u64,
    pub tls: Option<InboundTlsConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InboundConfig {
    pub protocol: SupportedProtocols,
    pub address: String,
    pub port: u16,
    // pub transport: Option<TransportProtocol>,
    pub tls: Option<InboundTlsConfig>,
}



#[derive(Serialize, Deserialize, Clone)]
pub struct InboundTlsConfig {
    pub cert_path: String,
    pub key_path: String,
}



impl Config {
    pub fn get_users_url(self) -> String {
        format!("{}/api/v1/server/trojan/users", self.webapi_url.trim_end_matches("/"))
    }

    pub fn get_submit_url(self) -> String {
        format!("{}/api/v1/server/trojan/submit", self.webapi_url.trim_end_matches("/"))
    }

    pub fn get_config_url(self) -> String {
        format!("{}/api/v1/server/trojan/config", self.webapi_url.trim_end_matches("/"))
    }

}