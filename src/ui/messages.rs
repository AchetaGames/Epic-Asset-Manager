use egs_api::api::types::account::UserData;
use egs_api::api::types::asset_info::{AssetInfo, KeyImage};
use egs_api::api::types::epic_asset::EpicAsset;

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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    DockerClient(ghregistry::Client),
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    GithubAuthFailed,
}
