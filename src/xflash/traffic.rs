use crate::config::base::Config;
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};
use tokio::{task, time};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Sender, Receiver};
use tokio::time::{sleep, Duration};
use log::{info, warn};
use byte_unit::{Byte};

static SLEEP_TIME:u64 = 30;
static DEFAULT_SIZE:usize = 32;
#[derive(Debug)]
pub struct TrafficCenter {
    store: Mutex<HashMap<i32,TrafficItem>>,
}

#[derive(Debug,  Deserialize, Serialize)]
pub struct TrafficItem {
    user_id: i32,
    u: u64,
    d: u64,
    n: u64
}

impl TrafficItem {
    pub fn new (user_id: i32, u: u64, d: u64, n: u64) -> TrafficItem {
        TrafficItem {
            user_id,
            u,
            d,
            n
        }
    }
}

impl TrafficCenter {
    pub fn new(store: Mutex<HashMap<i32,TrafficItem>>) -> Self { Self { store } }

    pub fn generate_channel() -> (Sender<TrafficItem>, Receiver<TrafficItem>) {
        mpsc::channel::<TrafficItem>(DEFAULT_SIZE)
    }

    pub async fn start_recevie_job(&'static self, mut receiver: Receiver<TrafficItem>) -> Result<()> {
        task::spawn(async move {
            info!("start traffic received job");
            while let Some(i) = receiver.recv().await {
                let mut store = self.store.lock().await;

                match store.get_mut(&i.user_id) {
                    Some(item) => {
                        item.u = item.u + i.u;
                        item.d = item.d + i.d;
                        item.n = item.n + i.n;
                    },
                    None => {
                        store.insert(i.user_id, i);
                    }
                }
            };
            info!("end traffic received job");
        });
        Ok(())
    }

    pub async fn start_submit(&'static self, config: Config) -> Result<()>  {
        let submit_interval = config.submit_interval;
        let params = [
            ("token", config.webapi_key.to_string()),
            ("node_id", config.node_id.to_string())
        ];
        let submit_with_parmas_url = match reqwest::Url::parse_with_params(config.get_submit_url().as_str(), &params) {
            Ok(url) => url,
            Err(e) => {
                return Err(anyhow!("Error parsing URL parameters : {}", e));
            }
        };
        info!("submit url :{}", submit_with_parmas_url);
        task::spawn(async move {
            sleep(Duration::from_secs(SLEEP_TIME)).await;
            let mut interval = time::interval(Duration::from_secs(submit_interval));

            loop {
                interval.tick().await;
                tokio::task:: unconstrained(async {
                    let mut store =  self.store.lock().await;
                    if  !store.is_empty()  {
                        let values = Vec::from_iter(store.values());

                        let ds: Vec<u64> = values.iter().map(|v|{
                            v.d  as u64
                        }).collect();

                        let ds_total = ds.iter().sum::<u64>();
                        let ds_total_byte = Byte::from_bytes(ds_total as u128);
                        info!("total down traffic: {}", ds_total_byte.get_appropriate_unit(true));

                        let us: Vec<u64> = values.iter().map(|v|{
                            v.u  as u64
                        }).collect();
                        let us_total = us.iter().sum::<u64>();
                        let us_total_byte = Byte::from_bytes(us_total as u128);
                        info!("total up traffic: {}", us_total_byte.get_appropriate_unit(true));

                        let response = match reqwest::Client::new().post(submit_with_parmas_url.as_str()).json(&values).send().await {
                            Ok(resp) => resp,
                            Err(e) => {
                                warn!("traffic map submit error: {}", e);
                                return;
                            },
                        };

                        if response.status().is_success() {
                            info!("traffic map submit success");
                        } else {
                            let response_status = response.status();
                            let response_text = response.text().await;
                            warn!("traffic map sumbmit error, status :{} , text : {}", response_status, response_text.unwrap());
                        }
                        store.clear();
                        info!("traffic map submited");
                    } else {
                        info!("traffic map is empty");
                    }
                }).await;
            }
        });
        Ok(())
    }
}

impl Default for TrafficCenter {
    fn default() -> Self {
        TrafficCenter{
            store: Mutex::new(HashMap::new())
        }
    }
}