use std::collections::HashMap;
use std::sync::Arc;
use egs_api::api::types::asset_info::AssetInfo;
use egs_api::api::types::epic_asset::EpicAsset;
use egs_api::api::types::download_manifest::DownloadManifest;

pub struct ApiData {
    pub asset_info: Arc<std::sync::RwLock<HashMap<String, AssetInfo>>>,
    pub asset_map: Arc<std::sync::RwLock<HashMap<String, EpicAsset>>>,
    pub asset_namespace_map: Arc<std::sync::RwLock<HashMap<String, Vec<String>>>>,
    pub download_manifests: Arc<std::sync::RwLock<HashMap<String, DownloadManifest>>>,
    pub tag_filter: Arc<std::sync::RwLock<Option<String>>>,
    pub search_filter: Arc<std::sync::RwLock<Option<String>>>,
}

impl ApiData {
    pub fn new() -> ApiData {
        ApiData {
            asset_info: Arc::new(Default::default()),
            asset_map: Arc::new(Default::default()),
            asset_namespace_map: Arc::new(Default::default()),
            download_manifests: Arc::new(Default::default()),
            tag_filter: Arc::new(Default::default()),
            search_filter: Arc::new(Default::default()),
        }
    }
}
