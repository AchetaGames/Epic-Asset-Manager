mod application;
#[rustfmt::skip]
mod config;
// mod download;
mod models;
mod tools;
mod ui;
mod window;

#[macro_use]
extern crate lazy_static;

use crate::config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR, PROFILE, RESOURCES_FILE, VERSION};
use application::EpicAssetManager;
use clap::{App, Arg};
use env_logger::Env;
use gettextrs::*;
use gtk4::gio;
use log::debug;
use std::io::Write;
use std::sync::Arc;

lazy_static! {
    static ref RUNNING: Arc<std::sync::RwLock<bool>> = Arc::new(std::sync::RwLock::new(true));
}

fn main() {
    #[cfg(windows)]
    {
        WindowsResource::new()
            .set_icon("data/icons/io.github.achetagames.epic_asset_manager.ico")
            .compile()?;
    }

    let matches = App::new("Epic Asset Manager")
        .about("A GUI tool to access the Epic Games Store Assets")
        .arg(
            Arg::with_name("v")
                .short("v")
                .help("Sets the level of verbosity"),
        )
        .get_matches();

    env_logger::Builder::from_env(Env::default().default_filter_or(
        match matches.occurrences_of("v") {
            0 => "epic_asset_manager:info",
            _ => "epic_asset_manager:debug",
        },
    ))
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
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).unwrap();
    textdomain(GETTEXT_PACKAGE).unwrap();

    gtk4::glib::set_application_name("Epic Asset Manager");
    gtk4::glib::set_prgname(Some("epic_asset_manager"));

    gtk4::init().expect("Unable to start GTK4");
    adw::init();

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = EpicAssetManager::new();
    debug!("{}", PKGDATADIR);
    debug!("{}", PROFILE);
    debug!("{}", VERSION);
    app.run();
    if let Ok(mut w) = crate::RUNNING.write() {
        *w = false
    }
}
