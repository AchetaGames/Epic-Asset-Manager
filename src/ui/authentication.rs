use crate::window::EpicAssetManagerWindow;
use chrono::{DateTime, Utc};
use gtk4::prelude::SettingsExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use log::{debug, warn};
use std::thread;

impl EpicAssetManagerWindow {
    /// Establish a Cosmos session after login so engine EULA checks and
    /// version queries can reuse the shared cookie jar without re-auth.
    fn setup_cosmos_session(eg: &mut egs_api::EpicGames) {
        if let Some(token) = crate::RUNTIME.block_on(eg.game_token()) {
            match crate::RUNTIME.block_on(eg.cosmos_session_setup(&token.code)) {
                Ok(_) => debug!("Cosmos session established"),
                Err(e) => warn!("Failed to establish Cosmos session: {:?}", e),
            }
        }
    }

    pub fn login(&self, sid: String) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = self.imp();
        self_.main_stack.set_visible_child_name("progress");
        self_.progress_message.set_text("Authenticating");
        let sender = self_.model.borrow().sender.clone();
        let s = sid;
        let mut eg = self_.model.borrow().epic_games.borrow().clone();
        thread::spawn(move || {
            let start = std::time::Instant::now();
            if crate::RUNTIME.block_on(eg.auth_code(None, Some(s))) {
                Self::setup_cosmos_session(&mut eg);
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
        crate::tools::auth::parse_token_time(self_.model.borrow().settings.string(key).as_str())
    }

    pub fn can_relogin(&self) -> bool {
        let self_: &crate::window::imp::EpicAssetManagerWindow = self.imp();
        let now = chrono::Utc::now();
        let user_details = self_.model.borrow().epic_games.borrow().user_details();
        crate::tools::auth::can_relogin(
            now,
            self.token_time("token-expiration"),
            user_details.access_token().is_some(),
            self.token_time("refresh-token-expiration"),
            user_details.refresh_token().is_some(),
        )
    }

    pub fn relogin(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = self.imp();
        let sender = self_.model.borrow().sender.clone();
        let mut eg = self_.model.borrow().epic_games.borrow().clone();
        thread::spawn(move || {
            let start = std::time::Instant::now();
            if crate::RUNTIME.block_on(eg.login()) {
                Self::setup_cosmos_session(&mut eg);
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
            crate::RUNTIME.block_on(eg.logout());
            sender
                .send_blocking(crate::ui::messages::Msg::Logout)
                .unwrap();
        });
    }
}
