use crate::ui::widgets::logged_in::refresh::Refresh;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub mod engines;
pub mod fab;
pub mod games;
pub mod library;
mod log_line;
pub mod logs;
mod plugins;
mod projects;
pub mod refresh;

pub mod imp {
    use gtk4::glib::{ParamSpec, ParamSpecString};
    use once_cell::sync::OnceCell;

    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/logged_in.ui")]
    pub struct EpicLoggedInBox {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        #[template_child]
        pub sidebar: TemplateChild<crate::ui::widgets::logged_in::library::sidebar::EpicSidebar>,
        #[template_child]
        pub page_stack: TemplateChild<gtk4::Stack>,
        #[template_child]
        pub library: TemplateChild<crate::ui::widgets::logged_in::library::EpicLibraryBox>,
        #[template_child]
        pub engines: TemplateChild<crate::ui::widgets::logged_in::engines::EpicEnginesBox>,
        #[template_child]
        pub projects: TemplateChild<crate::ui::widgets::logged_in::projects::EpicProjectsBox>,
        #[template_child]
        pub games: TemplateChild<crate::ui::widgets::logged_in::games::EpicGamesBox>,
        #[template_child]
        pub fab: TemplateChild<crate::ui::widgets::logged_in::fab::FabLibraryBox>,
        #[template_child]
        pub details:
            TemplateChild<crate::ui::widgets::logged_in::library::asset_detail::EpicAssetDetails>,
        pub settings: gtk4::gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicLoggedInBox {
        const NAME: &'static str = "EpicLoggedInBox";
        type Type = super::EpicLoggedInBox;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                sidebar: TemplateChild::default(),
                page_stack: TemplateChild::default(),
                library: TemplateChild::default(),
                engines: TemplateChild::default(),
                projects: TemplateChild::default(),
                games: TemplateChild::default(),
                fab: TemplateChild::default(),
                details: TemplateChild::default(),
                settings: gtk4::gio::Settings::new(crate::config::APP_ID),
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

    impl ObjectImpl for EpicLoggedInBox {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("item").build(),
                    ParamSpecString::builder("product").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "item" => {
                    let item: Option<String> = value.get().unwrap();
                    self.library.set_property("item", item);
                }
                "product" => {
                    let product: Option<String> = value.get().unwrap();
                    self.library.set_property("product", product);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "item" => self.library.property("item"),
                "product" => self.library.property("product"),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicLoggedInBox {}
    impl BoxImpl for EpicLoggedInBox {}
}

glib::wrapper! {
    pub struct EpicLoggedInBox(ObjectSubclass<imp::EpicLoggedInBox>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl Default for EpicLoggedInBox {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicLoggedInBox {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        self_.details.set_window(&window.clone());
        self_.library.set_details(&self_.details);
        self_.library.set_window(&window.clone());
        self_.library.set_sidebar(&self_.sidebar);
        self_.sidebar.set_page_stack(&self_.page_stack);
        self_.engines.set_window(&window.clone());
        self_.projects.set_window(&window.clone());
        self_.games.set_window(&window.clone());
        self_.fab.set_details(&self_.details);
        self_.fab.set_window(&window.clone());
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
        self_.details.set_download_manager(dm);
        self_.library.set_download_manager(dm);
        self_.engines.set_download_manager(dm);
        self_.fab.set_download_manager(dm);
    }

    pub fn details(&self) -> &library::asset_detail::EpicAssetDetails {
        let self_ = self.imp();
        &self_.details
    }

    pub fn update_docker(&self) {
        let self_ = self.imp();
        self_.engines.update_docker();
    }

    pub fn start_processing_asset(&self) {
        let self_ = self.imp();
        self_.library.start_processing_asset();
    }

    pub fn end_processing_asset(&self) {
        let self_ = self.imp();
        self_.library.end_processing_asset();
    }

    pub fn process_epic_asset(&self, epic_asset: &egs_api::api::types::epic_asset::EpicAsset) {
        let self_ = self.imp();
        self_.library.process_epic_asset(epic_asset);
    }

    pub fn load_thumbnail(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_ = self.imp();
        self_.library.load_thumbnail(asset);
    }

    pub fn add_asset(
        &self,
        asset: &egs_api::api::types::asset_info::AssetInfo,
        image: Option<gtk4::gdk::Texture>,
    ) {
        let self_ = self.imp();
        self_.library.add_asset(asset, image.clone());

        // Also add games to the games page
        if let Some(categories) = &asset.categories {
            for category in categories {
                if category.path.starts_with("games") || category.path.starts_with("dlc") {
                    self_.games.add_asset_info(asset, image);
                    break;
                }
            }
        }
    }

    pub fn flush_assets(&self) {
        let self_ = self.imp();
        self_.library.flush_assets();
    }

    pub fn add_fab_asset(
        &self,
        asset: &egs_api::api::types::fab_library::FabAsset,
        image: Option<gtk4::gdk::Texture>,
    ) {
        let self_ = self.imp();
        self_.fab.add_fab_asset(asset, image);
    }

    pub fn flush_fab_assets(&self) {
        let self_ = self.imp();
        self_.fab.flush_fab_assets();
    }

    pub fn activate(&self, _active: bool) {
        // No-op in unified view - all sections always visible
    }

    pub fn switch_page(&self, page: &str) {
        let self_ = self.imp();
        self_.page_stack.set_visible_child_name(page);
    }

    pub fn current_page(&self) -> Option<glib::GString> {
        let self_ = self.imp();
        self_.page_stack.visible_child_name()
    }
}

impl Refresh for EpicLoggedInBox {
    fn run_refresh(&self) {
        let self_ = self.imp();
        self_.engines.run_refresh();
        self_.projects.run_refresh();
        self_.library.run_refresh();
        self_.fab.run_refresh();
    }
}
