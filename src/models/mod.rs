pub mod asset_data;
pub mod category_data;
pub mod database;
pub mod engine_data;
pub mod log_data;
mod plugin_data;
pub mod project_data;

use crate::config::APP_ID;
use egs_api::EpicGames;
use gtk4::gio;
use gtk4::glib::UserDirectory;
use gtk4::prelude::*;
use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::thread;

#[cfg(target_os = "linux")]
use secret_service::{blocking::SecretService, EncryptionType};

pub struct Model {
    pub epic_games: RefCell<EpicGames>,
    #[cfg(target_os = "linux")]
    pub secret_service: Option<SecretService<'static>>,
    pub sender: async_channel::Sender<crate::ui::messages::Msg>,
    pub receiver: RefCell<Option<async_channel::Receiver<crate::ui::messages::Msg>>>,
    pub settings: gio::Settings,
    #[cfg(target_os = "linux")]
    pub dclient: RefCell<Option<ghregistry::Client>>,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    pub fn new() -> Self {
        let (sender, receiver) = async_channel::unbounded();
        let mut obj = Self {
            epic_games: RefCell::new(EpicGames::new()),
            #[cfg(target_os = "linux")]
            secret_service: match SecretService::connect(EncryptionType::Dh) {
                Ok(ss) => Some(ss),
                Err(e) => {
                    error!(
                        "Unable to initialize Secret service no secrets will be stored: {}",
                        e
                    );
                    None
                }
            },
            sender,
            receiver: RefCell::new(Some(receiver)),
            settings: gio::Settings::new(APP_ID),
            #[cfg(target_os = "linux")]
            dclient: RefCell::new(None),
        };
        obj.load_secrets();
        obj.load_defaults();
        obj
    }

    fn load_defaults(&mut self) {
        if self.settings.string("cache-directory").is_empty() {
            let mut dir = gtk4::glib::user_cache_dir();
            dir.push("epic_asset_manager");
            self.settings
                .set_string("cache-directory", dir.to_str().unwrap())
                .unwrap();
        }

        if self
            .settings
            .string("temporary-download-directory")
            .is_empty()
        {
            let mut dir = gtk4::glib::tmp_dir();
            dir.push("epic_asset_manager");
            self.settings
                .set_string("temporary-download-directory", dir.to_str().unwrap())
                .unwrap();
        }

        if self.settings.strv("unreal-projects-directories").is_empty() {
            match gtk4::glib::user_special_dir(UserDirectory::Documents) {
                None => { //TODO: Handle non standard directories
                }
                Some(mut dir) => {
                    dir.push("Unreal Projects");
                    self.settings
                        .set_strv("unreal-projects-directories", vec![dir.to_str().unwrap()])
                        .unwrap();
                }
            };
        }

        if self.settings.strv("unreal-vault-directories").is_empty() {
            match gtk4::glib::user_special_dir(UserDirectory::Documents) {
                None => {
                    //TODO: Handle non standard directories
                }
                Some(mut dir) => {
                    dir.push("EpicVault");
                    self.settings
                        .set_strv("unreal-vault-directories", vec![dir.to_str().unwrap()])
                        .unwrap();
                }
            };
        }

        if self.settings.strv("unreal-engine-directories").is_empty() {
            match gtk4::glib::user_special_dir(UserDirectory::Documents) {
                None => {
                    //TODO: Handle non standard directories
                }
                Some(mut dir) => {
                    dir.push("Unreal Engine");
                    self.settings
                        .set_strv("unreal-engine-directories", vec![dir.to_str().unwrap()])
                        .unwrap();
                }
            };
        }
    }

    pub fn validate_registry_login(&self, user: String, token: String) {
        debug!("Trying to validate token for {}", user);
        #[cfg(target_os = "linux")]
        {
            let client = ghregistry::Client::configure()
                .registry("ghcr.io")
                .insecure_registry(false)
                .username(Some(user))
                .password(Some(token))
                .build()
                .unwrap();
            let sender = self.sender.clone();
            thread::spawn(move || {
                let login_scope = "repository:epicgames/unreal-engine:pull";
                match client.authenticate(&[login_scope]) {
                    Ok(docker_client) => match docker_client.is_auth() {
                        Ok(auth) => {
                            if auth {
                                sender
                                    .send_blocking(crate::ui::messages::Msg::DockerClient(
                                        docker_client,
                                    ))
                                    .unwrap();
                                info!("Docker Authenticated");
                            }
                        }
                        Err(e) => {
                            error!("Failed authentication verification {:?}", e);
                        }
                    },
                    Err(e) => {
                        error!("Failed authentication {:?}", e);
                        sender
                            .send_blocking(crate::ui::messages::Msg::GithubAuthFailed)
                            .unwrap();
                    }
                };
            });
        }
    }

    fn load_secrets(&mut self) {
        #[cfg(target_os = "linux")]
        {
            match &self.secret_service {
                None => {
                    error!("Unable to load secrets from Secret service");
                    self.load_secrets_insecure();
                }
                Some(ss) => {
                    match ss.get_any_collection() {
                        Ok(collection) => {
                            match collection.search_items(HashMap::from([(
                                "application",
                                crate::config::APP_ID,
                            )])) {
                                Ok(items) => {
                                    let mut ud = egs_api::api::types::account::UserData::new();
                                    for item in items {
                                        let Ok(label) = item.get_label() else {
                                            debug!("No label skipping");
                                            continue;
                                        };
                                        debug!("Loading: {}", label);
                                        if let Ok(attributes) = item.get_attributes() {
                                            match label.as_str() {
                                                "eam_epic_games_token" => {
                                                    if let Some((token, t, exp)) = self
                                                        .load_egs_secrets(
                                                            &item,
                                                            &attributes,
                                                            "token-expiration",
                                                        )
                                                    {
                                                        ud.expires_at = Some(exp);
                                                        ud.token_type = Some(t);
                                                        ud.set_access_token(Some(token));
                                                    }
                                                }
                                                "eam_epic_games_refresh_token" => {
                                                    if let Some((token, _t, exp)) = self
                                                        .load_egs_secrets(
                                                            &item,
                                                            &attributes,
                                                            "refresh-token-expiration",
                                                        )
                                                    {
                                                        ud.refresh_expires_at = Some(exp);
                                                        ud.set_refresh_token(Some(token));
                                                    }
                                                }
                                                "eam_github_token" => {
                                                    if let Ok(d) = item.get_secret() {
                                                        if let Ok(s) =
                                                            std::str::from_utf8(d.as_slice())
                                                        {
                                                            self.validate_registry_login(
                                                                self.settings
                                                                    .string("github-user")
                                                                    .to_string(),
                                                                s.to_string(),
                                                            );
                                                        }
                                                    };
                                                }
                                                &_ => {}
                                            }
                                        }
                                    }
                                    self.epic_games.borrow_mut().set_user_details(ud);
                                }
                                Err(e) => {
                                    warn!("Unable to get items, trying insecure storage: {}", e);
                                    self.load_secrets_insecure();
                                }
                            };
                        }
                        Err(e) => {
                            warn!("Unable to get collection: {}", e);
                            self.load_secrets_insecure();
                        }
                    };
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            self.load_secrets_insecure();
        }
    }

    fn load_secrets_insecure(&self) {
        let mut ud = egs_api::api::types::account::UserData::new();
        if let Some((token, exp)) = self.load_egs_secrets_insecure("token") {
            ud.expires_at = Some(exp);
            ud.token_type = Some("bearer".to_string());
            ud.set_access_token(Some(token));
        };
        if let Some((token, exp)) = self.load_egs_secrets_insecure("refresh-token") {
            ud.refresh_expires_at = Some(exp);
            ud.set_refresh_token(Some(token));
        };
        self.epic_games.borrow_mut().set_user_details(ud);

        let gh_token = self.settings.string("github-token");
        if !gh_token.is_empty() {
            self.validate_registry_login(
                self.settings.string("github-user").to_string(),
                gh_token.to_string(),
            );
        }
    }

    fn load_egs_secrets(
        &self,
        item: &secret_service::blocking::Item,
        attributes: &std::collections::HashMap<String, String>,
        expiration: &str,
    ) -> Option<(String, String, chrono::DateTime<chrono::Utc>)> {
        let t = match attributes.get("type") {
            None => {
                debug!("Access token does not have type");
                return None;
            }
            Some(v) => v.clone(),
        };
        let exp =
            match chrono::DateTime::parse_from_rfc3339(self.settings.string(expiration).as_str()) {
                Ok(d) => d.with_timezone(&chrono::Utc),
                Err(e) => {
                    debug!("Failed to parse token expiration date {}", e);
                    return None;
                }
            };
        let now = chrono::offset::Utc::now();
        let td = exp - now;
        if td.num_seconds() < 600 {
            info!("Token {} is expired, removing", expiration);
            item.delete().unwrap_or_default();
            return None;
        }

        if let Ok(d) = item.get_secret() {
            if let Ok(s) = std::str::from_utf8(d.as_slice()) {
                debug!("Loaded {}", expiration);
                return Some((s.to_string(), t, exp));
            }
        };
        None
    }

    fn load_egs_secrets_insecure(
        &self,
        item: &str,
    ) -> Option<(String, chrono::DateTime<chrono::Utc>)> {
        let exp = match chrono::DateTime::parse_from_rfc3339(
            self.settings.string(&format!("{item}-expiration")).as_str(),
        ) {
            Ok(d) => d.with_timezone(&chrono::Utc),
            Err(e) => {
                debug!("Failed to parse token expiration date {}", e);
                return None;
            }
        };
        let now = chrono::offset::Utc::now();
        let td = exp - now;
        if td.num_seconds() < 600 {
            info!("Token {} is expired, removing", item);
            self.settings.set_string(item, "").unwrap();
            return None;
        }

        let secret = self.settings.string(item);
        if secret.is_empty() {
            None
        } else {
            Some((secret.to_string(), exp))
        }
    }
}
