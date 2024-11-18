mod application;
#[rustfmt::skip]
mod config;
// mod download;
mod models;
mod schema;
mod tools;
mod ui;
mod window;

use crate::config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR, PROFILE, RESOURCES_FILE, VERSION};
use application::EpicAssetManager;
use env_logger::Env;
use gettextrs::{bindtextdomain, setlocale, textdomain, LocaleCategory};
#[cfg(target_os = "linux")]
use gtk4::gio;
use log::debug;
use std::io::Write;
use std::sync::Arc;

lazy_static::lazy_static! {
    static ref RUNNING: Arc<std::sync::RwLock<bool>> = Arc::new(std::sync::RwLock::new(true));
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("epic_asset_manager:info"))
        .format(|buf, record| {
            writeln!(
                buf,
                "<{}> - [{}] - {}",
                record.target(),
                record.level(),
                record.args()
            )
        })
        .init();

    // Prepare i18n
    #[cfg(target_os = "linux")]
    {
        setlocale(LocaleCategory::LcAll, "");
        bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).unwrap();
        textdomain(GETTEXT_PACKAGE).unwrap();
    }

    gtk4::glib::set_application_name("Epic Asset Manager");
    gtk4::glib::set_prgname(Some("epic_asset_manager"));

    gtk4::init().expect("Unable to start GTK4");
    adw::init().expect("Unable to start Adwaita");

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = EpicAssetManager::new();
    debug!("{}", PKGDATADIR);
    debug!("{}", PROFILE);
    debug!("{}", VERSION);
    app.run();
    if let Ok(mut w) = crate::RUNNING.write() {
        *w = false;
    };
}
