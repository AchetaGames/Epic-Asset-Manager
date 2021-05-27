pub mod row_data;

use crate::config::APP_ID;
use egs_api::EpicGames;
use gtk::gio;
use gtk::glib::{MainContext, Receiver, Sender, UserDirectory, PRIORITY_DEFAULT};
use gtk::prelude::*;
use log::{debug, info};
use secret_service::{EncryptionType, SecretService};
use std::cell::RefCell;

// pub mod asset_model;
// pub mod row_data;
pub struct Model {
    pub epic_games: EpicGames,
    pub secret_service: SecretService<'static>,
    // asset_model: crate::models::asset_model::Model,
    // selected_asset: Option<String>,
    // selected_files: HashMap<String, HashMap<String, Vec<String>>>,
    // download_pool: ThreadPool,
    // thumbnail_pool: ThreadPool,
    // image_pool: ThreadPool,
    // file_pool: ThreadPool,
    // downloaded_chunks: HashMap<String, Vec<String>>,
    // downloaded_files: HashMap<String, DownloadedFile>,
    // download_manifest_tree: Tree<Option<CheckButton>>,
    // download_manifest_handlers: HashMap<NodeId, SignalHandlerId>,
    // download_manifest_file_details: HashMap<NodeId, (String, String, String, u128)>,
    pub sender: Sender<crate::ui::messages::Msg>,
    pub receiver: RefCell<Option<Receiver<crate::ui::messages::Msg>>>,
    // selected_files_size: u128,
    pub settings: gio::Settings,
}

impl Model {
    pub fn new() -> Self {
        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
        let mut obj = Self {
            epic_games: EpicGames::new(),
            secret_service: SecretService::new(EncryptionType::Dh)
                .expect("A running secret-service is required"),
            sender: sender.clone(),
            receiver: RefCell::new(Some(receiver)),
            settings: gio::Settings::new(APP_ID),
            // asset_model: crate::models::asset_model::Model::new(),
            // selected_asset: None,
            // selected_files: HashMap::new(),
            // download_pool: ThreadPool::with_name("Download Pool".to_string(), 5),
            // thumbnail_pool: ThreadPool::with_name("Thumbnail Pool".to_string(), 5),
            // image_pool: ThreadPool::with_name("Image Pool".to_string(), 5),
            // file_pool: ThreadPool::with_name("File Pool".to_string(), 5),
            // downloaded_chunks: HashMap::new(),
            // downloaded_files: HashMap::new(),
            // download_manifest_tree: TreeBuilder::new().with_root(None).build(),
            // download_manifest_handlers: HashMap::new(),
            // download_manifest_file_details: HashMap::new(),
            // selected_files_size: 0,
        };
        obj.load_secrets();
        obj.load_defaults();
        obj
    }

    fn load_defaults(&mut self) {
        if self.settings.string("cache-directory").is_empty() {
            let mut dir = gtk::glib::user_cache_dir();
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
            let mut dir = gtk::glib::tmp_dir();
            dir.push("epic_asset_manager");
            self.settings
                .set_string("temporary-download-directory", dir.to_str().unwrap())
                .unwrap();
        }

        if self.settings.strv("unreal-projects-directories").is_empty() {
            let mut dir = gtk::glib::user_special_dir(UserDirectory::Documents);
            dir.push("Unreal Projects");
            self.settings
                .set_strv("unreal-projects-directories", &[&dir.to_str().unwrap()])
                .unwrap();
        }

        if self.settings.strv("unreal-vault-directories").is_empty() {
            let mut dir = gtk::glib::user_special_dir(UserDirectory::Documents);
            dir.push("EpicVault");
            self.settings
                .set_strv("unreal-vault-directories", &[&dir.to_str().unwrap()])
                .unwrap();
        }

        if self.settings.strv("unreal-engine-directories").is_empty() {
            let mut dir = gtk::glib::user_special_dir(UserDirectory::Documents);
            dir.push("Unreal Engine");
            self.settings
                .set_strv("unreal-engine-directories", &[&dir.to_str().unwrap()])
                .unwrap();
        }
    }

    fn load_secrets(&mut self) {
        if let Ok(collection) = self.secret_service.get_default_collection() {
            if let Ok(items) = collection.search_items(
                [("application", crate::config::APP_ID)]
                    .iter()
                    .cloned()
                    .collect(),
            ) {
                let mut ud = egs_api::api::UserData::new();
                for item in items {
                    let label = match item.get_label() {
                        Ok(l) => l,
                        Err(_) => {
                            debug!("No label skipping");
                            continue;
                        }
                    };
                    debug!("Loading: {}", label);
                    if let Ok(attributes) = item.get_attributes() {
                        match label.as_str() {
                            "eam_epic_games_token" => {
                                let t = match attributes.get("type") {
                                    None => {
                                        debug!("Access token does not have type");
                                        continue;
                                    }
                                    Some(v) => v.clone(),
                                };
                                let exp = match chrono::DateTime::parse_from_rfc3339(
                                    self.settings.string("token-expiration").as_str(),
                                ) {
                                    Ok(d) => d.with_timezone(&chrono::Utc),
                                    Err(e) => {
                                        debug!("Failed to parse token expiration date {}", e);
                                        continue;
                                    }
                                };
                                let now = chrono::offset::Utc::now();
                                let td = exp - now;
                                if td.num_seconds() < 600 {
                                    info!("Token {} is expired, removing", label);
                                    item.delete().unwrap_or_default();
                                    continue;
                                }
                                ud.expires_at = Some(exp);
                                ud.token_type = Some(t);
                                if let Ok(d) = item.get_secret() {
                                    if let Ok(s) = std::str::from_utf8(d.as_slice()) {
                                        debug!("Loaded access token");
                                        ud.set_access_token(Some(s.to_string()))
                                    }
                                };
                            }
                            "eam_epic_games_refresh_token" => {
                                let exp = match chrono::DateTime::parse_from_rfc3339(
                                    self.settings.string("refresh-token-expiration").as_str(),
                                ) {
                                    Ok(d) => d.with_timezone(&chrono::Utc),
                                    Err(e) => {
                                        debug!(
                                            "Failed to parse refresh token expiration date {}",
                                            e
                                        );
                                        continue;
                                    }
                                };
                                let now = chrono::offset::Utc::now();
                                let td = exp - now;
                                if td.num_seconds() < 600 {
                                    info!("Token {} is expired, removing", label);
                                    item.delete().unwrap_or_default();
                                    continue;
                                }
                                ud.refresh_expires_at = Some(exp);
                                if let Ok(d) = item.get_secret() {
                                    if let Ok(s) = std::str::from_utf8(d.as_slice()) {
                                        debug!("Loaded refresh token");
                                        ud.set_refresh_token(Some(s.to_string()))
                                    }
                                };
                            }
                            &_ => {}
                        }
                    }
                }
                self.epic_games.set_user_details(ud);
            };
        };
    }
}
