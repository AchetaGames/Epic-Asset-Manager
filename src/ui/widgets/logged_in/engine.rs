use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub(crate) mod imp {
    use super::*;
    use gtk4::glib::{Object, ParamSpec};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/engine.ui")]
    pub struct EpicEngine {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        version: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        updatable: RefCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEngine {
        const NAME: &'static str = "EpicEngine";
        type Type = super::EpicEngine;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                version: RefCell::new(None),
                path: RefCell::new(None),
                updatable: RefCell::new(false),
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

    impl ObjectImpl for EpicEngine {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_boolean(
                        "needs-update",
                        "needs update",
                        "Check if engine needs update",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "version",
                        "Version",
                        "Version",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "path",
                        "Path",
                        "Path",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "needs-update" => {
                    let updatable = value.get().unwrap();
                    self.updatable.replace(updatable);
                }
                "version" => {
                    let version = value.get().unwrap();

                    self.version.replace(version);
                }
                "path" => {
                    let path = value.get().unwrap();

                    self.path.replace(path);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "needs-update" => self.updatable.borrow().to_value(),
                "version" => self.version.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicEngine {}
    impl BoxImpl for EpicEngine {}
}

glib::wrapper! {
    pub struct EpicEngine(ObjectSubclass<imp::EpicEngine>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicEngine {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicEngine = imp::EpicEngine::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_: &imp::EpicEngine = imp::EpicEngine::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }
}
