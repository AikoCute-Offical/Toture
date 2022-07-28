#[macro_use]
extern crate lazy_static;
use clap::{Command, Arg};
use log::{error, info};
use anyhow::{Result, anyhow};
use torture::config::base::{InboundConfig};
use torture::xflash::parse::fetch_inbound_config;
use torture::config::parse::reader_config;
use torture::xflash::user::{UserCenter};
use torture::xflash::traffic::{TrafficCenter, TrafficItem};
use tokio::sync::mpsc::{Sender};
use torture::proxy::server::TcpServer;
use env_logger;

lazy_static! {
    static ref USER_CENTER: UserCenter = UserCenter::default();
    static ref TRAFFIC_CENTER:  TrafficCenter = TrafficCenter::default();
}
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let matches = Command::new("Torture")
        .version(VERSION)
        .author("himanhimao")
        .about("Trojan Rust for xflash")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets the config file, read ./config/config.json by default")
                .takes_value(true),
        )
        .get_matches();

    let config_path = matches.value_of("config").unwrap_or("./config/config.json");

    info!("Parsing torture configuration from {}", config_path);

    let config = match reader_config(config_path) {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load config file, {}", e);
            return Err(anyhow!("Failed to load config file, {}", e ));
        }
    };

    let inbound_counfig = match fetch_inbound_config(config.clone()).await {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to fetch inbound config, {}", e);
            return Err(anyhow!("Failed to fetch inbound config, {}", e));
        }
    };
    USER_CENTER.start_fetch_users(config.clone()).await?;
    TRAFFIC_CENTER.start_submit(config.clone()).await?;
    let (traffic_sender, traffic_receiver) = TrafficCenter::generate_channel();
    TRAFFIC_CENTER.start_recevie_job(traffic_receiver).await?;
    start_tcp_server(inbound_counfig, traffic_sender).await?;
    Ok(())
}

async fn start_tcp_server(
    inbound_config: InboundConfig,
    traffic_sender: Sender<TrafficItem>,
) -> Result<()> {
    let server = match TcpServer::new(inbound_config) {
        Ok(server) => server,
        Err(e) => {
            error!("Failed to instantiate the server, {}", e);
            return Err(anyhow!("Failed to instantiate the server, {}", e));
        }
    };

    match server.start(&USER_CENTER, traffic_sender).await {
        Err(e) => info!("Server failure: {}, graceful shutdown", e.to_string()),
        Ok(()) => info!("Finished running server, exiting..."),
    }
    Ok(())
}
