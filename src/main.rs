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
use std::path::PathBuf;
use std::sync::Arc;

lazy_static::lazy_static! {
    static ref RUNNING: Arc<std::sync::RwLock<bool>> = Arc::new(std::sync::RwLock::new(true));
}

/// Find the gresource file in standard locations
fn find_resources_file() -> PathBuf {
    let resource_name = "resources.gresource";

    // Get the executable's directory
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    // List of paths to search (in order of priority)
    let mut search_paths: Vec<PathBuf> = vec![
        // 1. Configured path from build (for meson installs)
        PathBuf::from(RESOURCES_FILE),
        // 2. Relative to current directory (for development)
        PathBuf::from("data/resources/resources.gresource"),
    ];

    // 3. Next to the executable
    if let Some(ref dir) = exe_dir {
        search_paths.push(dir.join(resource_name));
        search_paths.push(dir.join("data/resources").join(resource_name));
    }

    // 4. User local install
    if let Some(home) = std::env::var_os("HOME") {
        let home_path = PathBuf::from(home);
        search_paths.push(
            home_path
                .join(".local/share/epic_asset_manager")
                .join(resource_name),
        );
    }

    // 5. System install locations
    search_paths.push(PathBuf::from("/usr/share/epic_asset_manager").join(resource_name));
    search_paths.push(PathBuf::from("/usr/local/share/epic_asset_manager").join(resource_name));

    // Find first existing file
    for path in &search_paths {
        if path.exists() {
            debug!("Found resources at: {:?}", path);
            return path.clone();
        }
    }

    // Fallback to configured path (will error with helpful message)
    PathBuf::from(RESOURCES_FILE)
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

    let resources_path = find_resources_file();
    let res = gio::Resource::load(&resources_path).unwrap_or_else(|e| {
        panic!(
            "Could not load gresource file at {:?}: {}",
            resources_path, e
        )
    });
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
