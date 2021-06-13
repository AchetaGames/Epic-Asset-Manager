use crate::ui::messages::Msg;
use crate::window::EpicAssetManagerWindow;
use log::debug;
use std::thread;

pub(crate) trait Update {
    fn update(&self, _event: Msg) {
        unimplemented!()
    }
}

impl Update for EpicAssetManagerWindow {
    fn update(&self, event: Msg) {
        let start = std::time::Instant::now();
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();

        match event.clone() {
            Msg::Open(_, _) => {}
            Msg::Quit => {}
            Msg::PasswordLogin => {}
            Msg::AlternateLogin => {}
            Msg::SidLogin => {}
            Msg::OpenBrowserSid => {}
            Msg::Login(_) => {}
            Msg::Logout => {}
            Msg::ShowLogin => self.show_login(),
            Msg::Relogin => {}
            Msg::LoginOk(ud) => self.show_assets(ud),
            Msg::ProcessAssetList(_, _) => {}
            Msg::ProcessAssetInfo(a) => {
                self_.logged_in_stack.load_thumbnail(a);
            }
            Msg::ProcessAssetThumbnail(a, i) => {
                self_.logged_in_stack.add_asset(a, i.as_slice());
            }
            Msg::DownloadImage(_, _) => {}
            Msg::LoadDownloadManifest(_, _) => {}
            Msg::ProcessDownloadManifest(_, _) => {}
            Msg::ProcessAssetSelected => {}
            Msg::FilterAssets(_) => {}
            Msg::SearchAssets => {}
            Msg::BindAssetModel => {}
            Msg::PulseProgress => {}
            Msg::CloseDetails => {}
            Msg::NextImage => {}
            Msg::PrevImage => {}
            Msg::ShowSettings(_) => {}
            Msg::ShowAssetDownload(_) => {}
            Msg::DownloadVersionSelected => {}
            Msg::ToggleAssetDownloadDetails => {}
            Msg::SelectForDownload(_, _, _, _, _) => {}
            Msg::DownloadAssets(_, _, _) => {}
            Msg::DownloadFileValidated(_, _, _, _) => {}
            Msg::DownloadProgressReport(_, _, _) => {}
            Msg::ExtractionFinished(_, _) => {}
            Msg::ConfigurationDirectorySelectionChanged(_) => {}
            Msg::ConfigurationAddUnrealEngineDir(_) => {}
            Msg::ConfigurationRemoveUnrealEngineDir(_, _) => {}
        }
        debug!(
            "{:?} - {} took {:?}",
            thread::current().id(),
            event,
            start.elapsed()
        );
    }
}
