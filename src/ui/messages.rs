use core::fmt;
use egs_api::api::types::asset_info::{AssetInfo, KeyImage, ReleaseInfo};
use egs_api::api::types::download_manifest::DownloadManifest;
use egs_api::api::types::epic_asset::EpicAsset;
use egs_api::api::UserData;
use relm_derive::Msg;
use std::collections::HashMap;
use std::path::PathBuf;
use webkit2gtk::LoadEvent;

#[derive(Msg, Debug, Clone)]
pub enum Msg {
    Quit,
    WebViewLoadFinished(LoadEvent),
    Login(String),
    Logout,
    ShowLogin,
    Relogin,
    LoginOk(UserData),
    ProcessAssetList(HashMap<String, Vec<String>>, HashMap<String, EpicAsset>),
    ProcessAssetInfo(AssetInfo),
    ProcessImage(Option<String>, Vec<u8>),
    DownloadImage(Option<String>, KeyImage),
    LoadDownloadManifest(String, ReleaseInfo),
    ProcessDownloadManifest(String, DownloadManifest),
    ProcessAssetSelected,
    FilterNone,
    FilterSome(String),
    Search,
    ApplyFilter,
    BindAssetModel,
    PulseProgress,
    CloseDetails,
    NextImage,
    PrevImage,
    ShowSettings(bool),
    ShowAssetDownload(bool),
    DownloadVersionSelected,
    ToggleAssetDownloadDetails,
    SelectForDownload(String, String, String),
    DownloadAssets(bool, String, String),
    DownloadProgressReport(String, u128, bool),
    ExtractionFinished(String, PathBuf),
    ConfigurationDirectorySelectionChanged(String),
}

impl fmt::Display for Msg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Msg::Quit => {
                write!(f, "Quit")
            }
            Msg::WebViewLoadFinished(_) => {
                write!(f, "WebViewLoadFinished")
            }
            Msg::Login(_) => {
                write!(f, "Login")
            }
            Msg::Relogin => {
                write!(f, "Relogin")
            }
            Msg::LoginOk(_) => {
                write!(f, "LoginOk")
            }
            Msg::ProcessAssetList(_, _) => {
                write!(f, "ProcessAssetList")
            }
            Msg::ProcessAssetInfo(_) => {
                write!(f, "ProcessAssetInfo")
            }
            Msg::ProcessImage(_, _) => {
                write!(f, "ProcessImage")
            }
            Msg::LoadDownloadManifest(_, _) => {
                write!(f, "LoadDownloadManifest")
            }
            Msg::ProcessDownloadManifest(_, _) => {
                write!(f, "ProcessDownloadManifest")
            }
            Msg::ProcessAssetSelected => {
                write!(f, "ProcessAssetSelected")
            }
            Msg::FilterNone => {
                write!(f, "FilterNone")
            }
            Msg::FilterSome(_) => {
                write!(f, "FilterSome")
            }
            Msg::Search => {
                write!(f, "Search")
            }
            Msg::ApplyFilter => {
                write!(f, "ApplyFilter")
            }
            Msg::BindAssetModel => {
                write!(f, "BindAssetModel")
            }
            Msg::PulseProgress => {
                write!(f, "PulseProgress")
            }
            Msg::CloseDetails => {
                write!(f, "CloseDetails")
            }
            Msg::DownloadImage(_, _) => {
                write!(f, "DownloadImage")
            }
            Msg::NextImage => {
                write!(f, "NextImage")
            }
            Msg::PrevImage => {
                write!(f, "PrevImage")
            }
            Msg::ShowSettings(_) => {
                write!(f, "ShowSettings")
            }
            Msg::ShowAssetDownload(_) => {
                write!(f, "ShowAssetDownload")
            }
            Msg::DownloadVersionSelected => {
                write!(f, "DownloadVersionSelected")
            }
            Msg::ToggleAssetDownloadDetails => {
                write!(f, "ToggleAssetDownloadDetails")
            }
            Msg::SelectForDownload(_, _, _) => {
                write!(f, "SelectForDownload")
            }
            Msg::DownloadAssets(_, _, _) => {
                write!(f, "DownloadAssets")
            }
            Msg::DownloadProgressReport(_, _, _) => {
                write!(f, "DownloadProgressReport")
            }
            Msg::ExtractionFinished(_, _) => {
                write!(f, "ExtractionFinished")
            }
            Msg::ConfigurationDirectorySelectionChanged(_) => {
                write!(f, "ConfigurationDirectorySelectionChanged")
            }
            Msg::Logout => {
                write!(f, "Logout")
            }
            Msg::ShowLogin => {
                write!(f, "ShowLogin")
            }
        }
    }
}
