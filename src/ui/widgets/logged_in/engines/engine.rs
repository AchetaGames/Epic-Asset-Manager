use gtk4::cairo::glib::SignalHandlerId;
use gtk4::{self, glib, glib::clone, prelude::*, subclass::prelude::*, CompositeTemplate};

pub mod imp {
    use super::*;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/engine.ui")]
    pub struct EpicEngine {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        version: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        guid: RefCell<Option<String>>,
        branch: RefCell<Option<String>>,
        updatable: RefCell<bool>,
        has_branch: RefCell<bool>,
        pub data: RefCell<Option<crate::models::engine_data::EngineData>>,
        pub handler: RefCell<Option<SignalHandlerId>>,
        #[template_child]
        pub logo: TemplateChild<adw::Avatar>,
        #[template_child]
        pub add: TemplateChild<adw::Avatar>,
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
                guid: RefCell::new(None),
                branch: RefCell::new(None),
                updatable: RefCell::new(false),
                has_branch: RefCell::new(false),
                data: RefCell::new(None),
                handler: RefCell::new(None),
                logo: TemplateChild::default(),
                add: TemplateChild::default(),
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
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("needs-update").build(),
                    ParamSpecString::builder("version").build(),
                    ParamSpecString::builder("path").build(),
                    ParamSpecString::builder("branch").build(),
                    ParamSpecBoolean::builder("has-branch").build(),
                    ParamSpecString::builder("guid").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "needs-update" => {
                    let updatable = value.get().unwrap();
                    self.updatable.replace(updatable);
                }
                "version" => {
                    let version = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("{l}"));
                    self.version.replace(version);
                }
                "path" => {
                    let path = value.get().unwrap();
                    self.path.replace(path);
                }
                "branch" => {
                    let branch = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("Branch: {l}"));
                    self.branch.replace(branch);
                }
                "has-branch" => {
                    let has_branch = value.get().unwrap();
                    self.has_branch.replace(has_branch);
                }
                "guid" => {
                    let guid = value.get().unwrap();
                    self.guid.replace(guid);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "needs-update" => self.updatable.borrow().to_value(),
                "version" => self.version.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "branch" => self.branch.borrow().to_value(),
                "has-branch" => self.has_branch.borrow().to_value(),
                "guid" => self.guid.borrow().to_value(),
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
        glib::Object::new()
    }

    pub fn path(&self) -> Option<String> {
        self.property("path")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
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
        let self_ = self.imp();
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn set_data(&self, data: &crate::models::engine_data::EngineData) {
        let self_ = self.imp();
        if let Some(d) = self_.data.take() {
            if let Some(id) = self_.handler.take() {
                d.disconnect(id);
            }
        }
        if data.valid() {
            self_.logo.set_visible(true);
            self_.add.set_visible(false);
        } else {
            self_.logo.set_visible(false);
            self_.add.set_visible(true);
        }

        self_.data.replace(Some(data.clone()));
        self.set_property("path", data.path());
        self.set_property("guid", data.guid());
        self.set_property("version", data.version());
        self.set_property("tooltip-text", data.path());
        self_.handler.replace(Some(data.connect_local(
            "finished",
            false,
            clone!(
                #[weak(rename_to=engine)]
                self,
                #[weak]
                data,
                #[upgrade_or]
                None,
                move |_| {
                    engine.finished(&data);
                    None
                }
            ),
        )));
    }

    fn finished(&self, data: &crate::models::engine_data::EngineData) {
        self.set_property("branch", data.branch());
        self.set_property("has-branch", data.has_branch());
        self.set_property("needs-update", data.needs_update());
    }
}
