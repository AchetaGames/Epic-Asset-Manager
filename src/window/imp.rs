use super::*;

#[derive(CompositeTemplate)]
#[template(resource = "/io/github/achetagames/epic_asset_manager/window.ui")]
pub struct EpicAssetManagerWindow {
    #[template_child]
    pub headerbar: TemplateChild<gtk::HeaderBar>,
    #[template_child]
    pub main_stack: TemplateChild<gtk::Stack>,
    #[template_child]
    pub logged_in_stack: TemplateChild<crate::ui::widgets::logged_in::EpicLoggedInBox>,
    pub settings: gio::Settings,
    // pub widgets: Widgets,
}

#[glib::object_subclass]
impl ObjectSubclass for EpicAssetManagerWindow {
    const NAME: &'static str = "EpicAssetManagerWindow";
    type Type = super::EpicAssetManagerWindow;
    type ParentType = gtk::ApplicationWindow;

    fn new() -> Self {
        // let builder =
        //     gtk::Builder::from_resource("/io/github/achetagames/epic_asset_manager/window.ui");
        // let main_stack: Stack = builder.object("main_stack").unwrap();
        // let logged_in_stack: Stack = builder.object("logged_in_stack").unwrap();
        // let login_box: Box = builder.object("login_box").unwrap();
        // let title_right_box: Box = builder.object("title_right_box").unwrap();
        // let progress_message = builder.object("progress_message").unwrap();
        // let asset_flow: FlowBox = builder.object("asset_flow").unwrap();
        // let all_button: Button = builder.object("all_button").unwrap();
        // let assets_button: Button = builder.object("assets_button").unwrap();
        // let plugins_button: Button = builder.object("plugins_button").unwrap();
        // let games_button: Button = builder.object("games_button").unwrap();
        // let settings_button: Button = builder.object("settings_button").unwrap();
        // let search: SearchEntry = builder.object("search").unwrap();
        // let progress_revealer: Revealer = builder.object("progress_revealer").unwrap();
        // let loading_progress: ProgressBar = builder.object("loading_progress").unwrap();
        // let details_revealer: Revealer = builder.object("details_revealer").unwrap();
        // let details_content: Box = builder.object("details_content").unwrap();
        // let close_details: Button = builder.object("close_details").unwrap();
        // let settings_close: Button = builder.object("settings_close").unwrap();
        // let download_button: Button = builder.object("download_button").unwrap();
        // let asset_download_details_close: Button =
        //     builder.object("asset_download_details_close").unwrap();
        //
        // let image_stack = Stack::new();
        // let login_entry: Entry = builder.object("username").unwrap();
        // let password_entry: Entry = builder.object("password").unwrap();
        // let sid_entry: Entry = builder.object("sid_entry").unwrap();
        // let login_button: Button = builder.object("login_button").unwrap();
        // let alternate_login: Button = builder.object("alternate_login").unwrap();
        // let sid_browser_button: Button = builder.object("sid_browser_button").unwrap();
        // let sid_login: Button = builder.object("sid_login").unwrap();
        // let sid_cancel: Button = builder.object("sid_cancel").unwrap();
        // let login_widgets = Login {
        //     login_entry,
        //     password_entry,
        //     sid_entry,
        // };
        //
        // let mut directory_selectors: HashMap<String, FileChooserWidget> = HashMap::new();
        // let cache: FileChooserWidget = builder.object("cache_directory_selector").unwrap();
        // directory_selectors.insert("cache_directory_selector".into(), cache);
        //
        // let temp: FileChooserWidget = builder.object("temp_download_directory_selector").unwrap();
        //
        // directory_selectors.insert("temp_download_directory_selector".into(), temp);
        //
        // let vault: FileChooserWidget = builder.object("ue_asset_vault_directory_selector").unwrap();
        //
        // directory_selectors.insert("ue_asset_vault_directory_selector".into(), vault);
        //
        // let ue_dir: FileChooserWidget = builder.object("ue_directory_selector").unwrap();
        // directory_selectors.insert("ue_directory_selector".into(), ue_dir);
        //
        // let ue_proj_dir: FileChooserWidget =
        //     builder.object("ue_project_directory_selector").unwrap();
        // directory_selectors.insert("ue_project_directory_selector".into(), ue_proj_dir);
        //
        // fs::create_dir_all(&model.configuration.directories.cache_directory).unwrap();
        // // directory_selectors
        // //     .get("cache_directory_selector")
        // //     .unwrap()
        // //     .set_filename(Some(&model.configuration.directories.cache_directory));
        //
        // fs::create_dir_all(&model.configuration.directories.temporary_download_directory).unwrap();
        // // directory_selectors
        // //     .get("temp_download_directory_selector")
        // //     .unwrap()
        // //     .set_filename(Some(
        // //         &model.configuration.directories.temporary_download_directory,
        // //     ));
        //
        // fs::create_dir_all(&model.configuration.directories.unreal_vault_directory).unwrap();
        // // directory_selectors
        // //     .get("ue_asset_vault_directory_selector")
        // //     .unwrap()
        // //     .set_filename(Some(
        // //         &model.configuration.directories.unreal_vault_directory,
        // //     ));
        //
        // let unreal_engine_directories_box: Box =
        //     builder.object("unreal_engine_directories_box").unwrap();
        //
        // let add_unreal_directory: Button = builder.object("add_unreal_directory").unwrap();
        //
        // let unreal_engine_project_directories_box: Box = builder
        //     .object("unreal_engine_project_directories_box")
        //     .unwrap();
        //
        // let add_unreal_project_directory: Button =
        //     builder.object("add_unreal_project_directory").unwrap();
        //
        // let settings_widgets = Settings {
        //     directory_selectors,
        //     unreal_engine_directories_box,
        //     unreal_engine_project_directories_box,
        // };
        //
        // let asset_version_combo: ComboBoxText = builder.object("asset_version_combo").unwrap();
        //
        // let asset_download_info_box: Box = builder.object("asset_download_info_box").unwrap();
        // let asset_download_content: Box = builder.object("asset_download_content").unwrap();
        // let asset_download_actions_box: Box = builder.object("download_actions").unwrap();
        // let download_asset_name: Label = builder.object("download_asset_name").unwrap();
        // let asset_download_info_revealer_button: Button = builder
        //     .object("asset_download_info_revealer_button")
        //     .unwrap();
        // let asset_download_info_revealer: Revealer =
        //     builder.object("asset_download_info_revealer").unwrap();
        // let asset_download_info_revealer_button_image: Image = builder
        //     .object("asset_download_info_revealer_button_image")
        //     .unwrap();
        //
        // let (download_progress_sender, recv) = MainContext::channel(PRIORITY_DEFAULT);
        // let s = sender.clone();
        // recv.attach(None, move |(chunk, progress, finished)| {
        //     s.send(crate::ui::messages::Msg::DownloadProgressReport(
        //         chunk, progress, finished,
        //     ));
        //     Continue(true)
        // });
        //
        // let asset_download_widgets = AssetDownloadDetails {
        //     asset_download_content,
        //     download_selected: None,
        //     download_asset_name,
        //     asset_version_combo,
        //     asset_download_info_box,
        //     asset_download_info_revealer_button,
        //     asset_download_info_revealer_button_image,
        //     asset_download_info_revealer,
        //     asset_download_actions_box,
        //     download_progress_sender,
        //     download_all: None,
        //     selected_files_size: Label::new(None),
        // };

        Self {
            headerbar: TemplateChild::default(),
            main_stack: TemplateChild::default(),
            logged_in_stack: TemplateChild::default(),
            settings: gio::Settings::new(crate::config::APP_ID),
        }
    }

    fn class_init(klass: &mut Self::Class) {
        Self::bind_template(klass);
    }

    // You must call `Widget`'s `init_template()` within `instance_init()`.
    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for EpicAssetManagerWindow {
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        // Devel Profile
        if PROFILE == "Devel" {
            obj.style_context().add_class("devel");
        }

        // load latest window state
        obj.load_window_size();
    }
}

impl WidgetImpl for EpicAssetManagerWindow {}

impl WindowImpl for EpicAssetManagerWindow {
    // save window state on delete event
    fn close_request(&self, obj: &Self::Type) -> Inhibit {
        if let Err(err) = obj.save_window_size() {
            warn!("Failed to save window state, {}", &err);
        }
        Inhibit(false)
    }
}

impl ApplicationWindowImpl for EpicAssetManagerWindow {}
