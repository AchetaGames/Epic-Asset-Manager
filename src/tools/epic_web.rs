use reqwest::blocking::{Client, ClientBuilder};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub(crate) struct EpicWeb {
    client: Client,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectResponse {
    pub redirect_url: String,
    pub authorization_code: Value,
    pub sid: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EULAResponse {
    pub data: Data,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    #[serde(rename = "Eula")]
    pub eula: Eula,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Eula {
    pub has_account_accepted: HasAccountAccepted,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HasAccountAccepted {
    pub accepted: bool,
    pub key: String,
    pub locale: String,
    pub version: i64,
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
                "https://www.unrealengine.com/id/api/set-sid?sid={}",
                sid
            ))
            .send()
        {
            error!("Failed to run query: {}", e);
        };
    }

    pub fn validate_eula(&self) -> bool {
        let mut map = HashMap::new();
        map.insert("query", "{    Eula {        hasAccountAccepted(id: \"unreal_engine\", locale: \"en\", accountId: \"8645b4947bbc4c0092a8b7236df169d1\"){            accepted            key            locale            version        }    }}");
        match self
            .client
            .post("https://graphql.unrealengine.com/ue/graphql")
            .json(&map)
            .send()
        {
            Err(e) => {
                error!("Failed to run query: {}", e);
            }
            Ok(r) => match r.json::<EULAResponse>() {
                Ok(eula) => {
                    return eula.data.eula.has_account_accepted.accepted;
                }
                Err(e) => {
                    error!("Failed to parse EULA json: {}", e)
                }
            },
        };
        false
    }

    pub fn run_query(&self, url: String) -> bool {
        match self.client.get(url).send() {
            Err(e) => {
                error!("Failed to run query: {}", e);
            }
            Ok(r) => {
                debug!("Got response: {:?}", r.text());
            }
        };
        true
    }
}
