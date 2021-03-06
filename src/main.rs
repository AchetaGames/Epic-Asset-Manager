#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
extern crate env_logger;
extern crate glib;

use crate::api_data::ApiData;
use crate::config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR, PROFILE, RESOURCES_FILE, VERSION};
use crate::configuration::Configuration;
use crate::download::DownloadedFile;
use egs_api::EpicGames;
use gettextrs::*;
use gio::prelude::*;
use glib::SignalHandlerId;
use gtk::prelude::{
    BuilderExtManual, ButtonExt, ComboBoxExt, FileChooserButtonExt, FileChooserExt, FlowBoxExt,
    SearchEntryExt, WidgetExt,
};
use gtk::traits::GtkWindowExt;
use gtk::{
    Application, ApplicationWindow, Box, Builder, Button, CheckButton, ComboBoxText, Entry,
    FileChooserButton, FlowBox, Image, Inhibit, Label, ProgressBar, Revealer, SearchEntry, Stack,
};
use relm::{connect, Channel, Relm, Sender, Widget};
use serde::{Deserialize, Serialize};
use slab_tree::{NodeId, Tree};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::{fs, str, thread};
use threadpool::ThreadPool;

mod api_data;
#[rustfmt::skip]
mod config;
mod configuration;
mod download;
mod models;
mod tools;
mod ui;

lazy_static! {
    static ref DATA: ApiData = ApiData::new();
    static ref MUTEX: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    static ref RUNNING: Arc<std::sync::RwLock<bool>> = Arc::new(std::sync::RwLock::new(true));
}

struct Model {
    relm: Relm<Win>,
    epic_games: EpicGames,
    configuration: Configuration,
    asset_model: crate::models::asset_model::Model,
    selected_asset: Option<String>,
    selected_files: HashMap<String, HashMap<String, Vec<String>>>,
    download_pool: ThreadPool,
    thumbnail_pool: ThreadPool,
    image_pool: ThreadPool,
    file_pool: ThreadPool,
    downloaded_chunks: HashMap<String, Vec<String>>,
    downloaded_files: HashMap<String, DownloadedFile>,
    download_manifest_tree: Tree<Option<CheckButton>>,
    download_manifest_handlers: HashMap<NodeId, SignalHandlerId>,
    download_manifest_file_details: HashMap<NodeId, (String, String, String, u128)>,
    selected_files_size: u128,
    application: Application,
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone)]
struct Widgets {
    window: ApplicationWindow,
    login_box: Box,
    main_stack: Stack,
    title_right_box: Box,
    progress_message: Label,
    asset_flow: FlowBox,
    search: SearchEntry,
    progress_revealer: Revealer,
    loading_progress: ProgressBar,
    details_revealer: Revealer,
    details_content: Box,
    close_details: Button,
    image_stack: Stack,
    logged_in_stack: Stack,
    settings_widgets: Settings,
    asset_download_widgets: AssetDownloadDetails,
    login_widgets: Login,
    download_button: Button,
}

impl Widgets {
    #[allow(dead_code)]
    fn get_window_size(&self) -> (i32, i32) {
        self.window.size()
    }
}

#[derive(Clone)]
struct Settings {
    directory_selectors: HashMap<String, FileChooserButton>,
    unreal_engine_directories_box: Box,
    unreal_engine_project_directories_box: Box,
}

#[derive(Clone)]
struct Login {
    login_entry: Entry,
    password_entry: Entry,
    sid_entry: Entry,
}

#[derive(Clone)]
struct AssetDownloadDetails {
    asset_version_combo: ComboBoxText,
    asset_download_info_box: Box,
    asset_download_info_revealer_button: Button,
    asset_download_info_revealer: Revealer,
    asset_download_info_revealer_button_image: Image,
    download_asset_name: Label,
    selected_files_size: Label,
    asset_download_content: Box,
    download_selected: Option<Button>,
    download_all: Option<Button>,
    download_progress_sender: Sender<(String, u128, bool)>,
    asset_download_actions_box: Box,
}

struct Win {
    model: Model,
    widgets: Widgets,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginResponse {
    #[serde(rename = "redirectUrl")]
    pub redirect_url: String,
    pub sid: String,
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = ApplicationWindow;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        debug!("Main thread id: {:?}", thread::current().id());

        info!("Starging GTK Window");
        let builder = Builder::from_resource("/io/github/achetagames/epic_asset_manager/window.ui");

        let window: ApplicationWindow = builder.object("window").unwrap();
        window.set_application(Some(&model.application));
        let main_stack: Stack = builder.object("main_stack").unwrap();
        let logged_in_stack: Stack = builder.object("logged_in_stack").unwrap();
        let login_box: Box = builder.object("login_box").unwrap();
        let title_right_box: Box = builder.object("title_right_box").unwrap();
        let progress_message = builder.object("progress_message").unwrap();
        let asset_flow: FlowBox = builder.object("asset_flow").unwrap();
        let all_button: Button = builder.object("all_button").unwrap();
        let assets_button: Button = builder.object("assets_button").unwrap();
        let plugins_button: Button = builder.object("plugins_button").unwrap();
        let games_button: Button = builder.object("games_button").unwrap();
        let settings_button: Button = builder.object("settings_button").unwrap();
        let search: SearchEntry = builder.object("search").unwrap();
        let progress_revealer: Revealer = builder.object("progress_revealer").unwrap();
        let loading_progress: ProgressBar = builder.object("loading_progress").unwrap();
        let details_revealer: Revealer = builder.object("details_revealer").unwrap();
        let details_content: Box = builder.object("details_content").unwrap();
        let close_details: Button = builder.object("close_details").unwrap();
        let settings_close: Button = builder.object("settings_close").unwrap();
        let download_button: Button = builder.object("download_button").unwrap();
        let asset_download_details_close: Button =
            builder.object("asset_download_details_close").unwrap();

        let image_stack = Stack::new();

        relm.stream().emit(ui::messages::Msg::BindAssetModel);

        connect!(
            relm,
            model.application,
            connect_open(_, f, s),
            ui::messages::Msg::Open(f.into(), s.to_string())
        );
        connect!(
            relm,
            search,
            connect_search_changed(_),
            ui::messages::Msg::SearchAssets
        );

        connect!(
            relm,
            all_button,
            connect_clicked(_),
            ui::messages::Msg::FilterAssets(None)
        );
        connect!(
            relm,
            settings_button,
            connect_clicked(_),
            ui::messages::Msg::ShowSettings(true)
        );
        connect!(
            relm,
            settings_close,
            connect_clicked(_),
            ui::messages::Msg::ShowSettings(false)
        );
        connect!(
            relm,
            download_button,
            connect_clicked(_),
            ui::messages::Msg::ShowAssetDownload(true)
        );
        connect!(
            relm,
            asset_download_details_close,
            connect_clicked(_),
            ui::messages::Msg::ShowAssetDownload(false)
        );
        connect!(
            relm,
            assets_button,
            connect_clicked(_),
            ui::messages::Msg::FilterAssets(Some("assets".to_string()))
        );
        connect!(
            relm,
            plugins_button,
            connect_clicked(_),
            ui::messages::Msg::FilterAssets(Some("plugins".to_string()))
        );
        connect!(
            relm,
            games_button,
            connect_clicked(_),
            ui::messages::Msg::FilterAssets(Some("games".to_string()))
        );
        connect!(
            relm,
            close_details,
            connect_clicked(_),
            ui::messages::Msg::CloseDetails
        );

        let login_entry: Entry = builder.object("username").unwrap();
        let password_entry: Entry = builder.object("password").unwrap();
        let sid_entry: Entry = builder.object("sid_entry").unwrap();
        let login_button: Button = builder.object("login_button").unwrap();
        let alternate_login: Button = builder.object("alternate_login").unwrap();
        let sid_browser_button: Button = builder.object("sid_browser_button").unwrap();
        let sid_login: Button = builder.object("sid_login").unwrap();
        let sid_cancel: Button = builder.object("sid_cancel").unwrap();
        connect!(
            relm,
            login_button,
            connect_clicked(_),
            ui::messages::Msg::PasswordLogin
        );
        connect!(
            relm,
            sid_cancel,
            connect_clicked(_),
            ui::messages::Msg::ShowLogin
        );
        connect!(
            relm,
            sid_login,
            connect_clicked(_),
            ui::messages::Msg::SidLogin
        );
        connect!(
            relm,
            sid_browser_button,
            connect_clicked(_),
            ui::messages::Msg::OpenBrowserSid
        );
        connect!(
            relm,
            alternate_login,
            connect_clicked(_),
            ui::messages::Msg::AlternateLogin
        );

        let login_widgets = Login {
            login_entry,
            password_entry,
            sid_entry,
        };
        match model.configuration.user_data {
            None => {
                relm.stream().emit(ui::messages::Msg::ShowLogin);
            }
            Some(_) => {
                relm.stream().emit(ui::messages::Msg::Relogin);
            }
        }
        connect!(
            relm,
            window,
            connect_delete_event(_, _),
            return (Some(ui::messages::Msg::Quit), Inhibit(false))
        );
        connect!(
            relm,
            asset_flow,
            connect_selected_children_changed(_),
            ui::messages::Msg::ProcessAssetSelected
        );

        // Settings
        let mut directory_selectors: HashMap<String, FileChooserButton> = HashMap::new();

        let cache: FileChooserButton = builder.object("cache_directory_selector").unwrap();
        connect!(
            relm,
            cache,
            connect_file_set(_),
            crate::ui::messages::Msg::ConfigurationDirectorySelectionChanged(
                "cache_directory_selector".to_string()
            )
        );
        directory_selectors.insert("cache_directory_selector".into(), cache);

        let temp: FileChooserButton = builder.object("temp_download_directory_selector").unwrap();
        connect!(
            relm,
            temp,
            connect_file_set(_),
            crate::ui::messages::Msg::ConfigurationDirectorySelectionChanged(
                "temp_download_directory_selector".to_string()
            )
        );
        directory_selectors.insert("temp_download_directory_selector".into(), temp);

        let vault: FileChooserButton = builder.object("ue_asset_vault_directory_selector").unwrap();
        connect!(
            relm,
            vault,
            connect_file_set(_),
            crate::ui::messages::Msg::ConfigurationDirectorySelectionChanged(
                "ue_asset_vault_directory_selector".to_string()
            )
        );
        directory_selectors.insert("ue_asset_vault_directory_selector".into(), vault);

        let ue_dir: FileChooserButton = builder.object("ue_directory_selector").unwrap();
        directory_selectors.insert("ue_directory_selector".into(), ue_dir);

        let ue_proj_dir: FileChooserButton =
            builder.object("ue_project_directory_selector").unwrap();
        directory_selectors.insert("ue_project_directory_selector".into(), ue_proj_dir);

        fs::create_dir_all(&model.configuration.directories.cache_directory).unwrap();
        directory_selectors
            .get("cache_directory_selector")
            .unwrap()
            .set_filename(&model.configuration.directories.cache_directory);

        fs::create_dir_all(&model.configuration.directories.temporary_download_directory).unwrap();
        directory_selectors
            .get("temp_download_directory_selector")
            .unwrap()
            .set_filename(&model.configuration.directories.temporary_download_directory);

        fs::create_dir_all(&model.configuration.directories.unreal_vault_directory).unwrap();
        directory_selectors
            .get("ue_asset_vault_directory_selector")
            .unwrap()
            .set_filename(&model.configuration.directories.unreal_vault_directory);

        let unreal_engine_directories_box: Box =
            builder.object("unreal_engine_directories_box").unwrap();

        let add_unreal_directory: Button = builder.object("add_unreal_directory").unwrap();
        connect!(
            relm,
            add_unreal_directory,
            connect_clicked(_),
            ui::messages::Msg::ConfigurationAddUnrealEngineDir("ue_directory_selector".to_string())
        );

        let unreal_engine_project_directories_box: Box = builder
            .object("unreal_engine_project_directories_box")
            .unwrap();

        let add_unreal_project_directory: Button =
            builder.object("add_unreal_project_directory").unwrap();
        connect!(
            relm,
            add_unreal_project_directory,
            connect_clicked(_),
            ui::messages::Msg::ConfigurationAddUnrealEngineDir(
                "ue_project_directory_selector".to_string()
            )
        );
        let settings_widgets = Settings {
            directory_selectors,
            unreal_engine_directories_box,
            unreal_engine_project_directories_box,
        };

        let asset_version_combo: ComboBoxText = builder.object("asset_version_combo").unwrap();

        let asset_download_info_box: Box = builder.object("asset_download_info_box").unwrap();
        let asset_download_content: Box = builder.object("asset_download_content").unwrap();
        let asset_download_actions_box: Box = builder.object("download_actions").unwrap();
        let download_asset_name: Label = builder.object("download_asset_name").unwrap();
        let asset_download_info_revealer_button: Button = builder
            .object("asset_download_info_revealer_button")
            .unwrap();
        let asset_download_info_revealer: Revealer =
            builder.object("asset_download_info_revealer").unwrap();
        let asset_download_info_revealer_button_image: Image = builder
            .object("asset_download_info_revealer_button_image")
            .unwrap();
        let stream = relm.stream().clone();
        let (_channel, download_progress_sender) =
            Channel::new(move |(chunk, progress, finished)| {
                stream.emit(ui::messages::Msg::DownloadProgressReport(
                    chunk, progress, finished,
                ));
            });

        connect!(
            relm,
            asset_version_combo,
            connect_changed(_),
            ui::messages::Msg::DownloadVersionSelected
        );
        connect!(
            relm,
            asset_download_info_revealer_button,
            connect_clicked(_),
            ui::messages::Msg::ToggleAssetDownloadDetails
        );
        let asset_download_widgets = AssetDownloadDetails {
            asset_download_content,
            download_selected: None,
            download_asset_name,
            asset_version_combo,
            asset_download_info_box,
            asset_download_info_revealer_button,
            asset_download_info_revealer_button_image,
            asset_download_info_revealer,
            asset_download_actions_box,
            download_progress_sender,
            download_all: None,
            selected_files_size: Label::new(None),
        };

        window.show_all();

        Win {
            model,
            widgets: Widgets {
                logged_in_stack,
                main_stack,
                title_right_box,
                login_box,
                window,
                progress_message,
                asset_flow,
                search,
                progress_revealer,
                loading_progress,
                details_revealer,
                details_content,
                close_details,
                image_stack,
                download_button,
                settings_widgets,
                asset_download_widgets,
                login_widgets,
            },
        }
    }
}

fn main() {
    // Prepare i18n
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).unwrap();
    textdomain(GETTEXT_PACKAGE).unwrap();
    #[cfg(windows)]
    {
        WindowsResource::new()
            .set_icon("data/icons/io.github.achetagames.epic_asset_manager.ico")
            .compile()?;
    }

    glib::set_application_name("Epic Asset Manager");
    glib::set_prgname(Some("epic_asset_manager"));

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");

    gio::resources_register(&res);

    let application =
        gtk::Application::new(Some(config::APP_ID), gio::ApplicationFlags::FLAGS_NONE);
    let win = Rc::new(RefCell::new(None));

    application.connect_startup(move |application| {
        let mut w = win.borrow_mut();
        *w = Some(relm::init::<Win>(application.clone()));
    });
    debug!("{}", PKGDATADIR);
    debug!("{}", PROFILE);
    debug!("{}", VERSION);

    application.run();
}
