use super::*;
use crate::ui::PreferencesWindow;

#[derive(Debug)]
pub struct EpicAssetManager {
    pub window: OnceCell<WeakRef<EpicAssetManagerWindow>>,
    pub settings: gio::Settings,
}

#[glib::object_subclass]
impl ObjectSubclass for EpicAssetManager {
    const NAME: &'static str = "EpicAssetManager";
    type Type = super::EpicAssetManager;
    type ParentType = gtk::Application;

    fn new() -> Self {
        let settings = gio::Settings::new(config::APP_ID);

        Self {
            window: OnceCell::new(),
            settings,
        }
    }
}

impl ObjectImpl for EpicAssetManager {}

impl gio::subclass::prelude::ApplicationImpl for EpicAssetManager {
    fn activate(&self, app: &Self::Type) {
        debug!("GtkApplication<EpicAssetManager>::activate");

        let priv_ = EpicAssetManager::from_instance(app);
        if let Some(window) = priv_.window.get() {
            let window = window.upgrade().unwrap();
            window.show();
            window.present();
            return;
        }

        let window = EpicAssetManagerWindow::new(app);
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
