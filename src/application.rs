use crate::config;
use crate::window::EpicAssetManagerWindow;
use gio::ApplicationFlags;
use glib::clone;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{gdk, gio, glib};
use gtk_macros::{action, stateful_action};
use log::{debug, info};
use once_cell::sync::OnceCell;

pub(crate) mod imp {
    use super::*;
    use crate::ui::PreferencesWindow;
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

    impl gio::subclass::prelude::ApplicationImpl for EpicAssetManager {
        fn open(&self, app: &Self::Type, files: &[gtk4::gio::File], _int: &str) {
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
            self.activate(app);
        }

        fn activate(&self, app: &Self::Type) {
            debug!("GtkApplication<EpicAssetManager>::activate");

            let priv_ = EpicAssetManager::from_instance(app);
            if let Some(window) = priv_.window.get() {
                window.show();

                if let Ok(item) = self.item.borrow().to_value().get::<String>() {
                    window.set_property("item", item).unwrap();
                }
                if let Ok(product) = self.product.borrow().to_value().get::<String>() {
                    window.set_property("product", product).unwrap();
                }
                self.product.replace(None);
                self.item.replace(None);
                window.present();
                return;
            }

            let mut window = EpicAssetManagerWindow::new(app);

            if let Ok(item) = self.item.borrow().to_value().get::<String>() {
                window.set_property("item", item).unwrap();
            }
            if let Ok(product) = self.product.borrow().to_value().get::<String>() {
                window.set_property("product", product).unwrap();
            }
            self.product.replace(None);
            self.item.replace(None);

            self.window
                .set(window.clone())
                .expect("Window already set.");

            window.check_login();
            window.present();
        }

        fn startup(&self, app: &Self::Type) {
            debug!("GtkApplication<EpicAssetManager>::startup");

            self.settings
                .connect_changed(Some("dark-mode"), |_settings, _key| {
                    let style_manager = adw::StyleManager::default().unwrap();
                    if style_manager.is_dark() {
                        style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
                    } else {
                        style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
                    }
                });

            self.parent_startup(app);

            adw::functions::init();

            app.set_resource_base_path(Some("/io/github/achetagames/epic_asset_manager"));
            Self::Type::setup_css();
            let app_d = app.downcast_ref::<super::EpicAssetManager>().unwrap();
            // Preferences
            action!(
                app_d,
                "preferences",
                clone!(@weak app as app => move |_,_| {
                    let preferences = PreferencesWindow::new();
                    preferences.set_transient_for(Some(app.main_window()));
                    preferences.set_window(app.main_window());
                    preferences.show();
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
        glib::Object::new(&[
            ("application-id", &Some(config::APP_ID)),
            ("flags", &ApplicationFlags::HANDLES_OPEN),
            (
                "resource-base-path",
                &Some("/io/github/achetagames/epic_asset_manager/"),
            ),
        ])
        .expect("Application initialization failed...")
    }

    pub fn main_window(&self) -> &EpicAssetManagerWindow {
        let self_ = crate::application::imp::EpicAssetManager::from_instance(self);
        self_.window.get().unwrap()
    }

    pub fn setup_gactions(&self) {
        let self_ = crate::application::imp::EpicAssetManager::from_instance(self);
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
                if let Ok(mut w) = crate::RUNNING.write() {
                    *w = false;
                }
                app.main_window().close();
                app.quit();
            })
        );

        let is_dark_mode = self_.settings.boolean("dark-mode");
        stateful_action!(
            self,
            "dark-mode",
            is_dark_mode,
            clone!(@weak self_.settings as settings =>  move |action, _| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let is_dark_mode = !action_state;
                action.set_state(&is_dark_mode.to_variant());
                if let Err(err) = settings.set_boolean("dark-mode", is_dark_mode) {
                    error!("Failed to switch dark mode: {} ", err);
                }
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
        let dialog = gtk4::AboutDialogBuilder::new()
            .program_name("Epic Asset Manager")
            .logo_icon_name(config::APP_ID)
            .license_type(gtk4::License::MitX11)
            .website("https://github.com/AchetaGames/Epic-Asset-Manager")
            .version(config::VERSION)
            .transient_for(self.main_window())
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
