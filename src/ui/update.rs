use crate::ui::messages::Msg;
use crate::window::EpicAssetManagerWindow;
use gtk4::prelude::SettingsExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use log::error;

pub trait Update {
    fn update(&self, _event: Msg) {
        unimplemented!()
    }
}

impl Update for EpicAssetManagerWindow {
    fn update(&self, event: Msg) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = self.imp();

        match event {
            Msg::ShowLogin => self.show_login(),
            Msg::LoginOk(ud) => {
                self.clear_notification("login");
                self.show_assets(&ud);
            }
            Msg::ProcessAssetInfo(a) => {
                self_.logged_in_stack.load_thumbnail(&a);
            }
            Msg::ProcessAssetThumbnail(a, i) => {
                self_.logged_in_stack.add_asset(&a, i);
            }
            Msg::DownloadImage(image, asset) => {
                self_.download_manager.download_thumbnail(
                    image,
                    asset,
                    self_.model.borrow().sender.clone(),
                );
            }
            Msg::FlushAssetThumbnails => {
                self_.logged_in_stack.flush_assets();
            }
            Msg::ProcessEpicAsset(epic_asset) => {
                self_.logged_in_stack.process_epic_asset(&epic_asset);
            }
            #[cfg(target_os = "linux")]
            Msg::DockerClient(dclient) => {
                self_.model.borrow_mut().dclient.replace(Some(dclient));
                self.clear_notification("GithubAuth");
                self_.logged_in_stack.update_docker();
            }
            #[cfg(target_os = "linux")]
            Msg::GithubAuthFailed => {
                self_.model.borrow_mut().dclient.replace(None);
                if !self_
                    .model
                    .borrow()
                    .settings
                    .string("github-user")
                    .is_empty()
                {
                    self.add_notification(
                        "GithubAuth",
                        "Github token authentication failed",
                        gtk4::MessageType::Warning,
                    );
                }

                self_.logged_in_stack.update_docker();
            }
            Msg::LoginFailed(reason) => {
                error!("{}", reason);
                self.add_notification("login", &reason, gtk4::MessageType::Warning);
                self.show_login();
            }
            Msg::Logout => self.do_logout(),
            Msg::StartAssetProcessing => {
                self_.logged_in_stack.start_processing_asset();
            }
            Msg::EndAssetProcessing => {
                self_.logged_in_stack.end_processing_asset();
            }
        }
    }
}
