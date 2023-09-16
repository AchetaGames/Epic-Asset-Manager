use crate::config;
use crate::window::EpicAssetManagerWindow;
use gio::ApplicationFlags;
use glib::clone;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{gdk, gio, glib};
use gtk_macros::action;
use log::{debug, error, info};
use once_cell::sync::OnceCell;

pub mod imp {
    use super::*;
    use log::error;
    use std::cell::RefCell;

    #[derive(Debug)]
    pub struct EpicAssetManager {
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub settings: gio::Settings,
        item: RefCell<Option<String>>,
        product: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicAssetManager {
        const NAME: &'static str = "EpicAssetManager";
        type Type = super::EpicAssetManager;
        type ParentType = gtk4::Application;

        fn new() -> Self {
            let settings = gio::Settings::new(config::APP_ID);

            Self {
                window: OnceCell::new(),
                settings,
                item: RefCell::new(None),
                product: RefCell::new(None),
            }
        }
    }

    impl ObjectImpl for EpicAssetManager {}

    impl ApplicationImpl for EpicAssetManager {
        fn activate(&self) {
            debug!("GtkApplication<EpicAssetManager>::activate");
            let app = self.obj();
            let self_ = app.imp();
            if let Some(window) = self_.window.get() {
                window.show();

                if let Ok(item) = self.item.borrow().to_value().get::<String>() {
                    window.set_property("item", item);
                }
                if let Ok(product) = self.product.borrow().to_value().get::<String>() {
                    window.set_property("product", product);
                }
                self.product.replace(None);
                self.item.replace(None);
                window.present();
                return;
            }

            let mut window = EpicAssetManagerWindow::new(&app);

            if let Ok(item) = self.item.borrow().to_value().get::<String>() {
                window.set_property("item", item);
            }
            if let Ok(product) = self.product.borrow().to_value().get::<String>() {
                window.set_property("product", product);
            }
            self.product.replace(None);
            self.item.replace(None);

            self.window
                .set(window.clone())
                .expect("Window already set.");

            window.check_login();
            window.present();
        }

        fn open(&self, files: &[gtk4::gio::File], _int: &str) {
            for file in files {
                if file.uri_scheme() == Some(gtk4::glib::GString::from("com.epicgames.launcher")) {
                    if let Some(asset) = file.basename() {
                        let name = asset
                            .file_name()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap_or_default();
                        let kind = file
                            .parent()
                            .unwrap()
                            .basename()
                            .unwrap_or_default()
                            .file_name()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap()
                            .to_string();
                        match kind.as_str() {
                            "product" => {
                                debug!("Trying to open product {}", name);
                                self.product.replace(Some(name.to_string()));
                                self.item.replace(None);
                            }
                            "item" => {
                                debug!("Trying to open item {}", name);
                                self.product.replace(None);
                                self.item.replace(Some(name.to_string()));
                            }
                            _ => {
                                self.product.replace(None);
                                self.item.replace(None);
                                error!("Please report what item in the store you clicked to get this response. {:?}", file.uri());
                            }
                        }
                    }
                }
            }
            self.activate();
        }

        fn startup(&self) {
            debug!("GtkApplication<EpicAssetManager>::startup");
            self.parent_startup();
            let app = self.obj();

            app.set_resource_base_path(Some("/io/github/achetagames/epic_asset_manager"));
            Self::Type::setup_css();
            let app_d = app.downcast_ref::<super::EpicAssetManager>().unwrap();
            // Preferences
            action!(
                app_d,
                "preferences",
                clone!(@weak app as app => move |_,_| {
                    app.main_window().show_preferences();
                })
            );

            app.setup_gactions();
            app.setup_accels();
        }
    }

    impl GtkApplicationImpl for EpicAssetManager {}
}

glib::wrapper! {
    pub struct EpicAssetManager(ObjectSubclass<imp::EpicAssetManager>)
        @extends gio::Application, gtk4::Application, @implements gio::ActionMap, gio::ActionGroup;
}

impl Default for EpicAssetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicAssetManager {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", config::APP_ID)
            .property("flags", ApplicationFlags::HANDLES_OPEN)
            .property(
                "resource-base-path",
                "/io/github/achetagames/epic_asset_manager/",
            )
            .build()
    }

    pub fn main_window(&self) -> &EpicAssetManagerWindow {
        let self_ = self.imp();
        self_.window.get().unwrap()
    }

    pub fn setup_gactions(&self) {
        let self_ = self.imp();
        self.connect_shutdown(|_| {
            if let Ok(mut w) = crate::RUNNING.write() {
                *w = false;
            }
        });

        // Quit
        action!(
            self,
            "quit",
            clone!(@weak self as app => move |_, _| {
                app.exit();
            })
        );

        let is_dark_mode = self_.settings.boolean("dark-mode");
        let simple_action =
            gio::SimpleAction::new_stateful("dark-mode", None, &is_dark_mode.to_variant());
        simple_action.connect_activate(clone!(@weak self as app =>  move |action, _| {
            app.toggle_dark_mode(action);
        }));
        self.add_action(&simple_action);

        // About
        action!(
            self,
            "about",
            clone!(@weak self as app => move |_, _| {
                app.show_about_dialog();
            })
        );

        let level = self_.settings.int("log-level");
        crate::ui::widgets::preferences::PreferencesWindow::set_log_level(level);
    }

    // Sets up keyboard shortcuts
    pub fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<primary>q"]);
        self.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
    }

    fn toggle_dark_mode(&self, action: &gtk4::gio::SimpleAction) {
        let self_ = self.imp();
        let state = action.state().unwrap();
        let action_state: bool = state.get().unwrap();
        let is_dark_mode = !action_state;
        action.set_state(&is_dark_mode.to_variant());
        if let Err(err) = self_.settings.set_boolean("dark-mode", is_dark_mode) {
            error!("Failed to switch dark mode: {} ", err);
        }
    }

    fn exit(&self) {
        if let Ok(mut w) = crate::RUNNING.write() {
            *w = false;
        }
        self.main_window().close();
        self.quit();
    }

    pub fn setup_css() {
        let provider = gtk4::CssProvider::new();
        provider.load_from_resource("/io/github/achetagames/epic_asset_manager/style.css");
        if let Some(display) = gdk::Display::default() {
            gtk4::StyleContext::add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn show_about_dialog(&self) {
        let dialog = gtk4::AboutDialog::builder()
            .program_name("Epic Asset Manager")
            .logo_icon_name(config::APP_ID)
            .license_type(gtk4::License::MitX11)
            .website("https://github.com/AchetaGames/Epic-Asset-Manager/wiki")
            .website_label("Wiki")
            .version(config::VERSION)
            .transient_for(self.main_window())
            .modal(true)
            .authors(vec!["Acheta Games".to_string()])
            .documenters(vec!["Osayami".to_string()])
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
