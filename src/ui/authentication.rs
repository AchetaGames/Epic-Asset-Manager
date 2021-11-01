use crate::window::EpicAssetManagerWindow;
use chrono::{DateTime, Utc};
use gtk4::prelude::SettingsExt;
use log::debug;
use std::thread;
use tokio::runtime::Runtime;

impl EpicAssetManagerWindow {
    pub fn login(&self, sid: String) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        self_.main_stack.set_visible_child_name("progress");
        self_.progress_message.set_text("Authenticating");
        let sender = self_.model.borrow().sender.clone();
        let s = sid;
        let mut eg = self_.model.borrow().epic_games.borrow().clone();
        thread::spawn(move || {
            let start = std::time::Instant::now();
            if let Some(exchange_token) = Runtime::new().unwrap().block_on(eg.auth_sid(s.as_str()))
            {
                if Runtime::new()
                    .unwrap()
                    .block_on(eg.auth_code(exchange_token))
                {
                    sender
                        .send(crate::ui::messages::Msg::LoginOk(eg.user_details()))
                        .unwrap();
                } else {
                    sender
                        .send(crate::ui::messages::Msg::LoginFailed(
                            "Unable to get auth code".to_string(),
                        ))
                        .unwrap();
                }
            } else {
                sender
                    .send(crate::ui::messages::Msg::LoginFailed(
                        "Unable to authenticate with sid".to_string(),
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
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        match chrono::DateTime::parse_from_rfc3339(
            self_.model.borrow().settings.string(key).as_str(),
        ) {
            Ok(d) => Some(d.with_timezone(&chrono::Utc)),
            Err(_) => None,
        }
    }

    pub fn can_relogin(&self) -> bool {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
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

    pub fn relogin(&mut self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        let sender = self_.model.borrow().sender.clone();
        let mut eg = self_.model.borrow().epic_games.borrow().clone();
        thread::spawn(move || {
            let start = std::time::Instant::now();
            if Runtime::new().unwrap().block_on(eg.login()) {
                sender
                    .send(crate::ui::messages::Msg::LoginOk(eg.user_details()))
                    .unwrap();
            } else {
                sender
                    .send(crate::ui::messages::Msg::LoginFailed(
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

    // fn logout(&mut self) {
    //     self.model.epic_games.set_user_details(UserData::new());
    //     self.widgets
    //         .title_right_box
    //         .foreach(|el| self.widgets.details_content.remove(el));
    //     let stream = self.model.relm.stream().clone();
    //     let (_channel, sender) = Channel::new(move |_| {
    //         stream.emit(crate::ui::messages::Msg::ShowLogin);
    //     });
    //
    //     let mut eg = self.model.epic_games.clone();
    //     thread::spawn(move || {
    //         let start = std::time::Instant::now();
    //         Runtime::new().unwrap().block_on(eg.logout());
    //         sender.send(true).unwrap();
    //         debug!(
    //             "{:?} - Logout requests took {:?}",
    //             thread::current().id(),
    //             start.elapsed()
    //         );
    //     });
    // }
}
