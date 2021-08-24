use super::*;
use crate::ui::PreferencesWindow;
use log::error;
use std::cell::RefCell;

#[derive(Debug)]
pub struct EpicAssetManager {
    pub window: OnceCell<WeakRef<EpicAssetManagerWindow>>,
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
                            error!("Please report what item in the store you clicked to get this response. {:?}", file.uri())
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
            let window = window.upgrade().unwrap();
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

        let window = EpicAssetManagerWindow::new(app);

        if let Ok(item) = self.item.borrow().to_value().get::<String>() {
            window.set_property("item", item).unwrap();
        }
        if let Ok(product) = self.product.borrow().to_value().get::<String>() {
            window.set_property("product", product).unwrap();
        }
        self.product.replace(None);
        self.item.replace(None);

        self.window
            .set(window.downgrade())
            .expect("Window already set.");

        app.main_window().check_login();
        app.main_window().present();
    }

    fn startup(&self, app: &Self::Type) {
        debug!("GtkApplication<EpicAssetManager>::startup");
        self.parent_startup(app);

        adw::functions::init();

        app.set_resource_base_path(Some("/io/github/achetagames/epic_asset_manager"));
        app.setup_css();
        let app_d = app.downcast_ref::<super::EpicAssetManager>().unwrap();
        // Preferences
        action!(
            app_d,
            "preferences",
            clone!(@weak app as app => move |_,_| {
                let preferences = PreferencesWindow::new();
                preferences.set_transient_for(Some(&app.main_window()));
                preferences.set_window(&app.main_window());
                preferences.show();
            })
        );

        app.setup_gactions();
        app.setup_accels();
    }
}

impl GtkApplicationImpl for EpicAssetManager {}
