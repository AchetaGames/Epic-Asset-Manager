use log::error;
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub struct EpicWeb {
    client: Client,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectResponse {
    pub redirect_url: String,
    pub authorization_code: Value,
    pub sid: String,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CosmosEulaAcceptResponse {
    pub accepted: bool,
}

impl EpicWeb {
    pub fn new() -> Self {
        let client = EpicWeb::build_client().build().unwrap();
        EpicWeb { client }
    }

    fn build_client() -> ClientBuilder {
        let mut headers = HeaderMap::new();
        headers.insert(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.0.0 Safari/537.36"
                .parse()
                .unwrap(),
        );
        Client::builder()
            .default_headers(headers)
            .cookie_store(true)
    }

    pub fn start_session(&mut self, exchange_token: String) {
        let mut csrf = String::new();
        match self
            .client
            .get("https://www.epicgames.com/id/api/reputation")
            .send()
        {
            Ok(r) => {
                for cookie in r.cookies() {
                    if cookie.name() == "XSRF-TOKEN" {
                        csrf = cookie.value().to_string();
                    }
                }
            }
            Err(e) => {
                error!("Failed to run query: {}", e);
            }
        }
        let mut map = HashMap::new();
        map.insert("exchangeCode", exchange_token);
        if let Err(e) = self
            .client
            .post("https://www.epicgames.com/id/api/exchange")
            .json(&map)
            .header("x-xsrf-token", csrf)
            .send()
        {
            error!("Failed to run query: {}", e);
        };
        let mut sid = String::new();
        match self
            .client
            .get("https://www.epicgames.com/id/api/redirect?")
            .send()
        {
            Ok(r) => match r.json::<RedirectResponse>() {
                Ok(response) => {
                    sid = response.sid;
                }
                Err(e) => {
                    error!("Error parsing json: {:?}", e);
                }
            },
            Err(e) => {
                error!("Failed to run query: {}", e);
            }
        }
        if let Err(e) = self
            .client
            .get(format!(
                "https://www.unrealengine.com/id/api/set-sid?sid={sid}"
            ))
            .send()
        {
            error!("Failed to run query: {}", e);
        };
    }

    pub fn validate_eula(&self) -> bool {
        match self
            .client
            .get("https://www.unrealengine.com/api/cosmos/eula/accept?eulaId=unreal_engine2&locale=en")
            .header("Accept", "application/json")
            .send()
        {
            Ok(r) => match r.json::<CosmosEulaAcceptResponse>() {
                Ok(response) => response.accepted,
                Err(e) => {
                    error!("Failed to parse EULA acceptance response: {}", e);
                    false
                }
            },
            Err(e) => {
                error!("Failed to query EULA acceptance: {}", e);
                false
            }
        }
    }

    pub fn run_query<T: DeserializeOwned>(&self, url: String) -> Result<T, reqwest::Error> {
        match self.client.get(url).send() {
            Err(e) => {
                error!("Failed to run query: {}", e);
                Err(e)
            }
            Ok(r) => r.json(),
        }
    }
}
