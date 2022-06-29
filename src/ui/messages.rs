use egs_api::api::types::asset_info::{AssetInfo, KeyImage};
use egs_api::api::types::epic_asset::EpicAsset;
use egs_api::api::UserData;

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
    #[cfg(target_os = "linux")]
    DockerClient(ghregistry::Client),
    #[cfg(target_os = "linux")]
    GithubAuthFailed,
}
