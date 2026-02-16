use egs_api::api::types::account::UserData;
use egs_api::api::types::asset_info::{AssetInfo, KeyImage};
use egs_api::api::types::epic_asset::EpicAsset;
use egs_api::api::types::fab_library::FabAsset;
use egs_api::api::types::fab_search::{FabListingDetail, FabListingUeFormat};
use egs_api::api::types::fab_taxonomy::FabTagGroup;

#[derive(Debug, Clone)]
pub enum Msg {
    ShowLogin,
    LoginOk(UserData),
    LoginFailed(String),
    Logout,
    StartAssetProcessing,
    EndAssetProcessing,
    ProcessAssetInfo(AssetInfo),
    ProcessEpicAsset(EpicAsset),
    ProcessAssetThumbnail(AssetInfo, Option<gtk4::gdk::Texture>),
    FlushAssetThumbnails,
    DownloadImage(KeyImage, AssetInfo),
    ProcessFabAsset(FabAsset, Option<gtk4::gdk::Texture>),
    FlushFabAssets,
    ProcessFabBrowseResult(FabAsset, Option<gtk4::gdk::Texture>),
    FlushFabBrowseResults(Option<String>),
    ProcessFabListingDetail(FabListingDetail, Vec<FabListingUeFormat>, bool),
    FabTaxonomyLoaded(Vec<FabTagGroup>),
    #[cfg(target_os = "linux")]
    DockerClient(ghregistry::Client),
    #[cfg(target_os = "linux")]
    GithubAuthFailed,
}
