use crate::proxy::base::SupportedProtocols::TROJAN;
use log::{info};
use crate::config::base::{Config, InboundConfig};
use anyhow::{Result, anyhow};

pub async fn fetch_inbound_config(config: Config) -> Result<InboundConfig> {
    let params = [
        ("token", config.webapi_key.to_string()),
        ("node_id", config.node_id.to_string()),
    ];
    let config_with_parmas_url = reqwest::Url::parse_with_params(config.clone().get_config_url().as_str(), &params)?;
    info!("connfig url :{}", config_with_parmas_url);

    let response=  reqwest::Client::new().get(config_with_parmas_url.clone()).send().await?;
    let json_result : serde_json::Value = response.json().await?;

    let json_data =  match json_result.get("data") {
        Some(data) => data,
        None =>  return Err(anyhow!("parse server data error")),
    };

    let server_port = match json_data.get("server_port") {
        Some(port) => port.as_u64().unwrap() as u16 ,
        None => return Err(anyhow!("parse server port error ")) , 
    };

    Ok(InboundConfig{
       protocol: TROJAN,
       address: String::from("0.0.0.0"),
       port: server_port ,
       tls: config.tls,
    })
}
