use crate::ui::messages::Msg;
use crate::window::EpicAssetManagerWindow;
use gtk::prelude::*;
use log::{debug, error};
use std::collections::HashMap;
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
            Msg::ShowLogin => {
                self.show_login();
            }
            Msg::Relogin => {}
            Msg::LoginOk(ud) => {
                self_.main_stack.set_visible_child_name("logged_in_stack");
                let collection = self_.model.secret_service.get_default_collection().unwrap();
                if let Some(t) = ud.token_type.clone() {
                    let mut attributes = HashMap::new();
                    attributes.insert("application", crate::config::APP_ID);
                    attributes.insert("type", t.as_str());
                    if let Some(e) = ud.expires_at.clone() {
                        let d = e.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                        self_
                            .model
                            .settings
                            .set_string("token-expiration", d.as_str())
                            .unwrap();
                        if let Some(at) = ud.access_token().clone() {
                            debug!("Saving token secret");
                            if let Err(e) = collection.create_item(
                                "eam_epic_games_token",
                                attributes.clone(),
                                at.as_bytes(),
                                true,
                                "text/plain",
                            ) {
                                error!("Failed to save secret {}", e)
                            };
                        }
                    }
                    let mut attributes = HashMap::new();
                    attributes.insert("application", crate::config::APP_ID);
                    attributes.insert("type", "refresh");
                    if let Some(e) = ud.refresh_expires_at.clone() {
                        let d = e.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                        self_
                            .model
                            .settings
                            .set_string("refresh-token-expiration", d.as_str())
                            .unwrap();
                        if let Some(rt) = ud.refresh_token().clone() {
                            debug!("Saving refresh token secret");
                            if let Err(e) = collection.create_item(
                                "eam_epic_games_refresh_token",
                                attributes,
                                rt.as_bytes(),
                                true,
                                "text/plain",
                            ) {
                                error!("Failed to save secret {}", e)
                            };
                        }
                    }
                }
            }
            Msg::ProcessAssetList(_, _) => {}
            Msg::ProcessAssetInfo(a) => {
                self_.logged_in_stack.load_thumbnail(a);
            }
            Msg::ProcessImage(a, i) => {
                self_.logged_in_stack.add_asset(a, i);
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
