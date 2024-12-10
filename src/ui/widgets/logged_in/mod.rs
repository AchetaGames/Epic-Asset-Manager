use crate::gio::glib::GString;
use crate::ui::widgets::logged_in::refresh::Refresh;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub mod engines;
pub mod library;
mod log_line;
pub mod logs;
mod plugins;
mod projects;
pub mod refresh;

pub mod imp {
    use std::cell::RefCell;

    use gtk4::glib::{ParamSpec, ParamSpecObject, ParamSpecString};
    use once_cell::sync::OnceCell;

    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/logged_in.ui")]
    pub struct EpicLoggedInBox {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        #[template_child]
        pub library: TemplateChild<crate::ui::widgets::logged_in::library::EpicLibraryBox>,
        #[template_child]
        pub engines: TemplateChild<crate::ui::widgets::logged_in::engines::EpicEnginesBox>,
        #[template_child]
        pub projects: TemplateChild<crate::ui::widgets::logged_in::projects::EpicProjectsBox>,
        #[template_child]
        pub adwstack: TemplateChild<adw::ViewStack>,
        pub settings: gtk4::gio::Settings,
        stack: RefCell<Option<adw::ViewStack>>,
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
                library: TemplateChild::default(),
                engines: TemplateChild::default(),
                projects: TemplateChild::default(),
                adwstack: TemplateChild::default(),
                settings: gtk4::gio::Settings::new(crate::config::APP_ID),
                stack: RefCell::new(None),
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
                    ParamSpecObject::builder::<adw::ViewStack>("stack").build(),
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
                "stack" => {
                    let stack = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.stack.replace(stack);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "item" => self.library.property("item"),
                "product" => self.library.property("product"),
                "stack" => self.stack.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicLoggedInBox {}
    impl BoxImpl for EpicLoggedInBox {}
}

glib::wrapper! {
    pub struct EpicLoggedInBox(ObjectSubclass<imp::EpicLoggedInBox>)
        @extends gtk4::Widget, gtk4::Box;
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
        self_.library.set_window(&window.clone());
        self_.engines.set_window(&window.clone());
        self_.projects.set_window(&window.clone());

        match self_.settings.string("default-view").as_str() {
            "engines" => self_.adwstack.set_visible_child_name("engines"),
            "projects" => self_.adwstack.set_visible_child_name("projects"),
            _ => self_.adwstack.set_visible_child_name("library"),
        }

        self_.adwstack.connect_visible_child_notify(clone!(
            #[weak(rename_to=li)]
            self,
            move |_| {
                li.tab_switched();
            }
        ));
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
        self_.library.set_download_manager(dm);
        self_.engines.set_download_manager(dm);
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
        self_.library.add_asset(asset, image);
    }

    pub fn flush_assets(&self) {
        let self_ = self.imp();
        self_.library.flush_assets();
    }

    fn active_page(&self) -> Option<GString> {
        let self_ = self.imp();
        self_.adwstack.visible_child_name()
    }

    pub fn activate(&self, active: bool) {
        let self_ = self.imp();
        if active {
            self.set_property("stack", &*self_.adwstack);
        } else {
            self.set_property("stack", None::<adw::ViewStack>);
        }
    }

    pub fn tab_switched(&self) {
        let self_ = self.imp();
        let available = if let Some(page) = self.active_page() {
            match page.as_str() {
                "library" => self_.library.can_be_refreshed(),
                "projects" => self_.projects.can_be_refreshed(),
                "engines" => self_.engines.can_be_refreshed(),
                _ => return,
            }
        } else {
            return;
        };
        if let Some(w) = self_.window.get() {
            let w_ = w.imp();
            w_.refresh.set_sensitive(available);
        }
    }

    pub fn switch_tab(&self, name: &str) {
        let self_ = self.imp();
        self_.adwstack.set_visible_child_name(name);
    }
}

impl Refresh for EpicLoggedInBox {
    fn run_refresh(&self) {
        let self_ = self.imp();
        if let Some(page) = self.active_page() {
            match page.as_str() {
                "library" => self_.library.run_refresh(),
                "projects" => self_.projects.run_refresh(),
                "engines" => self_.engines.run_refresh(),
                _ => {}
            }
        }
    }
}
