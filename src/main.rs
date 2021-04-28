mod application;
#[rustfmt::skip]
mod config;
mod configuration;
// mod download;
mod models;
mod tools;
mod ui;
mod window;

use crate::config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR, PROFILE, RESOURCES_FILE, VERSION};
use application::EpicAssetManager;
use gettextrs::*;
use gtk::gio;
use log::debug;

fn main() {
    #[cfg(windows)]
    {
        WindowsResource::new()
            .set_icon("data/icons/io.github.achetagames.epic_asset_manager.ico")
            .compile()?;
    }

    // Prepare i18n
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).unwrap();
    textdomain(GETTEXT_PACKAGE).unwrap();

    gtk::glib::set_application_name("Epic Asset Manager");
    gtk::glib::set_prgname(Some("epic_asset_manager"));

    gtk::init().expect("Unable to start GTK4");

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = EpicAssetManager::new();
    debug!("{}", PKGDATADIR);
    debug!("{}", PROFILE);
    debug!("{}", VERSION);
    app.run();
}
