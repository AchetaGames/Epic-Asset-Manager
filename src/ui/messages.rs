use core::fmt;
use egs_api::api::types::asset_info::{AssetInfo, KeyImage};
use egs_api::api::types::epic_asset::EpicAsset;
use egs_api::api::UserData;

#[derive(Debug, Clone)]
pub enum Msg {
    ShowLogin,
    LoginOk(UserData),
    LoginFailed(String),
    Logout,
    ProcessAssetInfo(AssetInfo),
    ProcessEpicAsset(EpicAsset),
    ProcessAssetThumbnail(AssetInfo, Vec<u8>),
    FlushAssetThumbnails,
    DownloadImage(KeyImage, AssetInfo),
    #[cfg(target_os = "linux")]
    DockerClient(ghregistry::Client),
    #[cfg(target_os = "linux")]
    GithubAuthFailed,
}

impl fmt::Display for Msg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Msg::LoginOk(_) => {
                write!(f, "LoginOk")
            }
            Msg::ProcessAssetInfo(_) => {
                write!(f, "ProcessAssetInfo")
            }
            Msg::ProcessAssetThumbnail(_, _) => {
                write!(f, "ProcessAssetThumbnail")
            }
            Msg::DownloadImage(_, _) => {
                write!(f, "DownloadImage")
            }
            Msg::ShowLogin => {
                write!(f, "ShowLogin")
            }
            Msg::FlushAssetThumbnails => {
                write!(f, "FlushAssetThumbnails")
            }
            Msg::ProcessEpicAsset(_) => {
                write!(f, "ProcessEpicAsset")
            }
            #[cfg(target_os = "linux")]
            Msg::DockerClient(_) => {
                write!(f, "DockerClient")
            }
            #[cfg(target_os = "linux")]
            Msg::GithubAuthFailed => {
                write!(f, "GithubAuthFailed")
            }
            Msg::LoginFailed(_) => {
                write!(f, "LoginFailed")
            }
            Msg::Logout => {
                write!(f, "Logout")
            }
        }
    }
}
