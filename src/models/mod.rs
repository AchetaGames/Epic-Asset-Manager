use crate::config::APP_ID;
use crate::configuration::Configuration;
use egs_api::EpicGames;
use gtk::glib::{MainContext, Receiver, Sender, SignalHandlerId, PRIORITY_DEFAULT};
use gtk::{gio, CheckButton};
use slab_tree::{NodeId, Tree, TreeBuilder};
use std::collections::HashMap;
use threadpool::ThreadPool;

// pub mod asset_model;
// pub mod row_data;
#[derive(Debug)]
pub struct Model {
    epic_games: EpicGames,
    pub configuration: Configuration,
    // asset_model: crate::models::asset_model::Model,
    selected_asset: Option<String>,
    selected_files: HashMap<String, HashMap<String, Vec<String>>>,
    download_pool: ThreadPool,
    thumbnail_pool: ThreadPool,
    image_pool: ThreadPool,
    file_pool: ThreadPool,
    downloaded_chunks: HashMap<String, Vec<String>>,
    // downloaded_files: HashMap<String, DownloadedFile>,
    download_manifest_tree: Tree<Option<CheckButton>>,
    download_manifest_handlers: HashMap<NodeId, SignalHandlerId>,
    download_manifest_file_details: HashMap<NodeId, (String, String, String, u128)>,
    sender: Sender<crate::ui::messages::Msg>,
    receiver: Receiver<crate::ui::messages::Msg>,
    selected_files_size: u128,
    pub settings: gio::Settings,
}

impl Model {
    pub fn new() -> Self {
        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
        Self {
            epic_games: EpicGames::new(),
            configuration: Configuration::new(),
            // asset_model: crate::models::asset_model::Model::new(),
            selected_asset: None,
            selected_files: HashMap::new(),
            download_pool: ThreadPool::with_name("Download Pool".to_string(), 5),
            thumbnail_pool: ThreadPool::with_name("Thumbnail Pool".to_string(), 5),
            image_pool: ThreadPool::with_name("Image Pool".to_string(), 5),
            file_pool: ThreadPool::with_name("File Pool".to_string(), 5),
            downloaded_chunks: HashMap::new(),
            // downloaded_files: HashMap::new(),
            download_manifest_tree: TreeBuilder::new().with_root(None).build(),
            download_manifest_handlers: HashMap::new(),
            download_manifest_file_details: HashMap::new(),
            sender: sender.clone(),
            receiver,
            selected_files_size: 0,
            settings: gio::Settings::new(APP_ID),
        }
    }
}
