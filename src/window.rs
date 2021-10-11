use crate::application::EpicAssetManager;
use crate::config::{APP_ID, PROFILE};
use crate::ui::update::Update;
use crate::ui::widgets::progress_icon::ProgressIconExt;
use glib::clone;
use glib::signal::Inhibit;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{gio, glib, CompositeTemplate};
use gtk_macros::action;
use log::{debug, error, warn};
use std::collections::HashMap;
use std::ops::Deref;

pub(crate) mod imp;

glib::wrapper! {
    pub struct EpicAssetManagerWindow(ObjectSubclass<imp::EpicAssetManagerWindow>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow, gio::ActionMap, gio::ActionGroup;
}

impl EpicAssetManagerWindow {
    pub fn new(app: &EpicAssetManager) -> Self {
        let window: Self = glib::Object::new(&[]).expect("Failed to create EpicAssetManagerWindow");
        window.set_application(Some(app));

        gtk4::Window::set_default_icon_name(APP_ID);

        window
    }

    pub fn data(&self) -> &imp::EpicAssetManagerWindow {
        imp::EpicAssetManagerWindow::from_instance(self)
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();

        let settings = &self_.model.borrow().settings;

        let size = self.default_size();

        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;

        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();

        let settings = &self_.model.borrow().settings;

        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let is_maximized = settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    pub fn setup_receiver(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        self_
            .model
            .borrow()
            .deref()
            .receiver
            .borrow_mut()
            .take()
            .unwrap()
            .attach(
                None,
                clone!(@weak self as window => @default-panic, move |msg| {
                    window.update(msg);
                    glib::Continue(true)
                }),
            );
    }

    pub fn setup_actions(&self) {
        action!(
            self,
            "login",
            Some(&String::static_variant_type()),
            clone!(@weak self as window => move |_, sid_par| {
                if let Some(sid_opt) = sid_par {
                    if let Some(sid) = sid_opt.get::<String>() {
                        window.login(sid);
                    }
                }
            })
        );
        let self_: &imp::EpicAssetManagerWindow = imp::EpicAssetManagerWindow::from_instance(self);

        self_.download_manager.connect_local(
            "tick",
            false,
            clone!(@weak self as obj => @default-return None, move |_| {
                let self_: &imp::EpicAssetManagerWindow = imp::EpicAssetManagerWindow::from_instance(&obj);
                self_.progress_icon.set_fraction(self_.download_manager.progress());
                None}),
        )
            .unwrap();
    }

    pub fn check_login(&mut self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        self_.main_stack.set_visible_child_name("progress");
        self_.progress_message.set_text("Loading");
        if self.can_relogin() {
            self_.progress_message.set_text("Resuming session");
            self.relogin();
        } else {
            self.show_login();
        }
    }

    pub fn show_login(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        self_.sid_box.set_window(self);
        self_.logged_in_stack.activate(false);
        self_.main_stack.set_visible_child_name("sid_box")
    }

    pub fn show_download_manager(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        self_.logged_in_stack.activate(false);
        // self_.main_stack.set_visible_child_name("download_manager")
    }

    pub fn show_logged_in(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        self_.logged_in_stack.activate(true);
        self_.main_stack.set_visible_child_name("logged_in_stack");
    }

    pub fn show_assets(&self, ud: ::egs_api::api::UserData) {
        // TODO display user information from the UserData
        let self_: &crate::window::imp::EpicAssetManagerWindow =
            crate::window::imp::EpicAssetManagerWindow::from_instance(self);
        self_
            .model
            .borrow_mut()
            .epic_games
            .borrow_mut()
            .set_user_details(ud.clone());
        self_.logged_in_stack.set_window(self);
        self_.download_manager.set_window(self);
        self_
            .logged_in_stack
            .set_download_manager(&self_.download_manager);
        self.show_logged_in();
        if let Some(t) = ud.token_type.clone() {
            let mut attributes = HashMap::new();
            attributes.insert("application", crate::config::APP_ID);
            attributes.insert("type", t.as_str());
            if let Some(e) = ud.expires_at {
                let d = e.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                self_
                    .model
                    .borrow()
                    .settings
                    .set_string("token-expiration", d.as_str())
                    .unwrap();
                if let Some(at) = ud.access_token() {
                    debug!("Saving token secret");
                    if let Err(e) = self_
                        .model
                        .borrow()
                        .secret_service
                        .get_default_collection()
                        .unwrap()
                        .create_item(
                            "eam_epic_games_token",
                            attributes.clone(),
                            at.as_bytes(),
                            true,
                            "text/plain",
                        )
                    {
                        error!("Failed to save secret {}", e)
                    };
                }
            }
            let mut attributes = HashMap::new();
            attributes.insert("application", crate::config::APP_ID);
            attributes.insert("type", "refresh");
            if let Some(e) = ud.refresh_expires_at {
                let d = e.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                self_
                    .model
                    .borrow()
                    .settings
                    .set_string("refresh-token-expiration", d.as_str())
                    .unwrap();
                if let Some(rt) = ud.refresh_token() {
                    debug!("Saving refresh token secret");
                    if let Err(e) = self_
                        .model
                        .borrow()
                        .secret_service
                        .get_default_collection()
                        .unwrap()
                        .create_item(
                            "eam_epic_games_refresh_token",
                            attributes,
                            rt.as_bytes(),
                            true,
                            "text/plain",
                        )
                    {
                        error!("Failed to save secret {}", e)
                    };
                }
            }
        }
        self.show_logged_in();
        self_.logged_in_stack.set_window(self);
        self_.download_manager.set_window(self);
        self_
            .logged_in_stack
            .set_download_manager(&self_.download_manager);
    }
}
