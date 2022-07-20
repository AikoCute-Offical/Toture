use crate::config::base::Config;
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::{task, time};
use sha2::{Digest, Sha224};
use std::collections::HashMap;
use log::{warn, debug, info};

#[derive(Debug)]
pub struct UserCenter {
    store: RwLock<HashMap<Vec<u8>, i32>>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct OriginalUserItem {
    id: i32,
    uuid: String,
}

#[derive(Debug, Clone)]
struct UserItem {
    id: i32,
    secret: Vec<u8>,
}


#[derive(Debug, Deserialize, Serialize)]
struct UsersResult {
    data: Vec<OriginalUserItem>,
}

impl OriginalUserItem {
    pub fn as_user_item(&self) -> UserItem {
        UserItem {
            id: self.id,
            secret: Sha224::digest(self.uuid.as_bytes())
                .iter()
                .map(|x| format!("{:02x}", x))
                .collect::<String>()
                .as_bytes()
                .to_vec(),
        }
    }
}

impl UserCenter {
    pub fn new(store: RwLock<HashMap<Vec<u8>, i32>>) -> Self { Self { store } }

    pub async fn start_fetch_users(&'static self, config: Config) -> Result<()> {
        let check_interval = config.check_interval;
        let params = [
            ("token", config.webapi_key.to_string()),
            ("node_id", config.node_id.to_string()),
        ];
        let users_with_parmas_url = match reqwest::Url::parse_with_params(config.get_users_url().as_str(), &params) {
            Ok(url) => url,
            Err(e) => {
                return Err(anyhow!("Error parsing URL parameters : {}", e));
            }
        };
        debug!("fetch users url: {}", users_with_parmas_url);
        task::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(check_interval));
            loop {
                interval.tick().await;
                tokio::task:: unconstrained(async {
                    let response  = match reqwest::Client::new()
                        .get(users_with_parmas_url.as_str())
                        .send()
                        .await {
                            Err(e) => {
                                warn!("fetch users error: {}", e);
                                return;
                            },
                            Ok(resp) => resp,
                        };

                    let result:UsersResult = match response.json().await {
                        Err(e) => {
                            warn!("users decode error: {}", e);
                            return;
                        },
                        Ok(result) => result,
                    };

                    let mut store = self.store.write().await;
                    store.clear();
                    for item in result.data.iter() {
                        let  user_item = item.as_user_item();
                        store.insert(user_item.secret, user_item.id);
                    }
                    info!("latest number of users: {}", store.len());
                }).await;
            }
        });
        Ok(())
    }

    #[inline]
    pub async fn validate(&'static self, secret: &[u8]) -> (bool, i32) {
        let users = self.store.read().await;
        if users.contains_key(secret) {
            match users.get(secret) {
                Some(number) => return (true, *number),
                _ => warn!("invalid user secret: {}", String::from_utf8_lossy(secret))
            }
        }
        (false, -1)
    }
}

impl Default for UserCenter {
    fn default() -> Self {
         UserCenter {
            store: RwLock::new(HashMap::new()),
        }
    }
}
