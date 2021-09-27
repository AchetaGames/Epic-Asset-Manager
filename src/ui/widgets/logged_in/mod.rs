mod asset;
pub mod asset_detail;
pub mod category;
mod download_detail;
pub mod engine;
mod engines;
pub mod image_stack;
pub mod library;
mod project;
mod project_detail;
mod projects;

use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub(crate) mod imp {
    use super::*;
    use gtk4::glib::ParamSpec;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/logged_in.ui")]
    pub struct EpicLoggedInBox {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        #[template_child]
        pub library: TemplateChild<crate::ui::widgets::logged_in::library::EpicLibraryBox>,
        #[template_child]
        pub engine: TemplateChild<crate::ui::widgets::logged_in::engines::EpicEnginesBox>,
        #[template_child]
        pub projects: TemplateChild<crate::ui::widgets::logged_in::projects::EpicProjectsBox>,
        #[template_child]
        pub stack: TemplateChild<adw::ViewStack>,
        item: RefCell<Option<String>>,
        product: RefCell<Option<String>>,
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
                engine: TemplateChild::default(),
                projects: TemplateChild::default(),
                stack: TemplateChild::default(),
                item: RefCell::new(None),
                product: RefCell::new(None),
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
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_string(
                        "item",
                        "item",
                        "item",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "product",
                        "product",
                        "product",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "item" => {
                    let item: Option<String> = value.get().unwrap();
                    self.library.set_property("item", item).unwrap();
                }
                "product" => {
                    let product: Option<String> = value.get().unwrap();
                    self.library.set_property("product", product).unwrap();
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "item" => self
                    .library
                    .property("item")
                    .unwrap_or_else(|_| "".to_value()),
                "product" => self
                    .library
                    .property("product")
                    .unwrap_or_else(|_| "".to_value()),
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
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        self_.library.set_window(&window.clone());
        self_.engine.set_window(&window.clone());
        self_.projects.set_window(&window.clone());
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
        self_.library.set_download_manager(&dm.clone());
    }

    pub(crate) fn process_epic_asset(
        &self,
        epic_asset: &egs_api::api::types::epic_asset::EpicAsset,
    ) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        self_.library.process_epic_asset(epic_asset);
    }

    pub fn load_thumbnail(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        self_.library.load_thumbnail(asset)
    }

    pub fn add_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo, image: &[u8]) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        self_.library.add_asset(asset, image);
    }

    pub fn flush_assets(&self) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        self_.library.flush_assets();
    }
}
