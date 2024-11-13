use crate::window::EpicAssetManagerWindow;
use chrono::{DateTime, Utc};
use gtk4::prelude::SettingsExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use log::debug;
use std::thread;
use tokio::runtime::Builder;

impl EpicAssetManagerWindow {
    pub fn login(&self, sid: String) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = self.imp();
        self_.main_stack.set_visible_child_name("progress");
        self_.progress_message.set_text("Authenticating");
        let sender = self_.model.borrow().sender.clone();
        let s = sid;
        let mut eg = self_.model.borrow().epic_games.borrow().clone();
        thread::spawn(move || {
            let start = std::time::Instant::now();
            if Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(eg.auth_code(None, Some(s)))
            {
                sender
                    .send_blocking(crate::ui::messages::Msg::LoginOk(eg.user_details()))
                    .unwrap();
            } else {
                sender
                    .send_blocking(crate::ui::messages::Msg::LoginFailed(
                        "Unable to authenticate with auth code".to_string(),
                    ))
                    .unwrap();
            };
            debug!(
                "{:?} - Login requests took {:?}",
                thread::current().id(),
                start.elapsed()
            );
        });
    }

    pub fn token_time(&self, key: &str) -> Option<DateTime<Utc>> {
        let self_: &crate::window::imp::EpicAssetManagerWindow = self.imp();
        chrono::DateTime::parse_from_rfc3339(self_.model.borrow().settings.string(key).as_str())
            .map_or(None, |d| Some(d.with_timezone(&chrono::Utc)))
    }

    pub fn can_relogin(&self) -> bool {
        let self_: &crate::window::imp::EpicAssetManagerWindow = self.imp();
        let now = chrono::Utc::now();
        if let Some(te) = self.token_time("token-expiration") {
            let td = te - now;
            if td.num_seconds() > 600
                && self_
                    .model
                    .borrow()
                    .epic_games
                    .borrow()
                    .user_details()
                    .access_token()
                    .is_some()
            {
                debug!("Access token is valid and exists");
                return true;
            }
        }
        if let Some(rte) = self.token_time("refresh-token-expiration") {
            let td = rte - now;
            if td.num_seconds() > 600
                && self_
                    .model
                    .borrow()
                    .epic_games
                    .borrow()
                    .user_details()
                    .refresh_token()
                    .is_some()
            {
                debug!("Refresh token is valid and exists");
                return true;
            }
        }
        false
    }

    pub fn relogin(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = self.imp();
        let sender = self_.model.borrow().sender.clone();
        let mut eg = self_.model.borrow().epic_games.borrow().clone();
        thread::spawn(move || {
            let start = std::time::Instant::now();
            if Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(eg.login())
            {
                sender
                    .send_blocking(crate::ui::messages::Msg::LoginOk(eg.user_details()))
                    .unwrap();
            } else {
                sender
                    .send_blocking(crate::ui::messages::Msg::LoginFailed(
                        "Relogin request failed".to_string(),
                    ))
                    .unwrap();
            };
            debug!(
                "{:?} - Relogin requests took {:?}",
                thread::current().id(),
                start.elapsed()
            );
        });
    }

    pub fn logout(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = self.imp();
        let sender = self_.model.borrow().sender.clone();
        let mut eg = self_.model.borrow().epic_games.borrow().clone();

        thread::spawn(move || {
            Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(eg.logout());
            sender
                .send_blocking(crate::ui::messages::Msg::Logout)
                .unwrap();
        });
    }
}
