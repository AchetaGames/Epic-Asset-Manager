use crate::config;
use crate::ui::widgets::preferences::window::PreferencesWindow;
use crate::window::EpicAssetManagerWindow;
use gio::ApplicationFlags;
use glib::clone;
use glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib};
use gtk_macros::action;
use log::{debug, info};
use once_cell::sync::OnceCell;

pub(crate) mod imp;

glib::wrapper! {
    pub struct EpicAssetManager(ObjectSubclass<imp::EpicAssetManager>)
        @extends gio::Application, gtk::Application, @implements gio::ActionMap, gio::ActionGroup;
}

impl EpicAssetManager {
    pub fn new() -> Self {
        glib::Object::new(&[
            ("application-id", &Some(config::APP_ID)),
            ("flags", &ApplicationFlags::empty()),
        ])
        .expect("Application initialization failed...")
    }

    pub fn get_main_window(&self) -> EpicAssetManagerWindow {
        let priv_ = crate::application::imp::EpicAssetManager::from_instance(self);
        priv_.window.get().unwrap().upgrade().unwrap()
    }

    pub fn setup_gactions(&self) {
        // Quit
        action!(
            self,
            "quit",
            clone!(@weak self as app => move |_, _| {
                // This is needed to trigger the delete event
                // and saving the window state
                app.get_main_window().close();
                app.quit();
            })
        );

        // Quit
        action!(
            self,
            "inspector",
            clone!(@weak self as app => move |_, _| {
                // This is needed to trigger the delete event
                // and saving the window state
                gtk::Window::set_interactive_debugging(true);
            })
        );

        // About
        action!(
            self,
            "about",
            clone!(@weak self as app => move |_, _| {
                app.show_about_dialog();
            })
        );
    }

    // Sets up keyboard shortcuts
    pub fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<primary>q"]);
        self.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
    }

    pub fn setup_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/io/github/achetagames/epic_asset_manager/style.css");
        if let Some(display) = gdk::Display::default() {
            gtk::StyleContext::add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    pub fn init(&self) {
        let priv_ = crate::application::imp::EpicAssetManager::from_instance(self);
        match priv_.model.configuration.user_data {
            None => priv_
                .window
                .get()
                .unwrap()
                .upgrade()
                .unwrap()
                .data()
                .main_stack
                .set_visible_child_name("sid_box"),
            Some(_) => priv_
                .window
                .get()
                .unwrap()
                .upgrade()
                .unwrap()
                .data()
                .main_stack
                .set_visible_child_name("logged_in_stack"),
        }
    }

    fn show_about_dialog(&self) {
        let dialog = gtk::AboutDialogBuilder::new()
            .program_name("Epic Asset Manager")
            .logo_icon_name(config::APP_ID)
            .license_type(gtk::License::MitX11)
            .website("https://github.com/AchetaGames/Epic-Asset-Manager")
            .version(config::VERSION)
            .transient_for(&self.get_main_window())
            .modal(true)
            .authors(vec!["Milan Stastny".into()])
            .build();

        dialog.show();
    }

    pub fn run(&self) {
        info!("Epic Asset Manager ({})", config::APP_ID);
        info!("Version: {} ({})", config::VERSION, config::PROFILE);
        info!("Datadir: {}", config::PKGDATADIR);

        ApplicationExtManual::run(self);
    }
}
