use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;

pub(crate) mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/create_asset_project.ui")]
    pub struct EpicCreateAssetProject {
        selected_version: RefCell<Option<String>>,
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub settings: gio::Settings,
        #[template_child]
        pub select_target_directory: TemplateChild<gtk4::ComboBoxText>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicCreateAssetProject {
        const NAME: &'static str = "EpicCreateAssetProject";
        type Type = super::EpicCreateAssetProject;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                selected_version: RefCell::new(None),
                asset: RefCell::new(None),
                actions: gio::SimpleActionGroup::new(),
                download_manager: OnceCell::new(),
                settings: gio::Settings::new(crate::config::APP_ID),
                select_target_directory: TemplateChild::default(),
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

    impl ObjectImpl for EpicCreateAssetProject {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
            obj.set_target_directories();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder(
                        "start-download",
                        &[],
                        <()>::static_type().into(),
                    )
                    .flags(glib::SignalFlags::ACTION)
                    .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecString::new(
                    "selected-version",
                    "selected_version",
                    "selected_version",
                    None, // Default value
                    glib::ParamFlags::READWRITE,
                )]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "selected-version" => {
                    let selected_version = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.selected_version.replace(selected_version);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "selected-version" => self.selected_version.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicCreateAssetProject {}
    impl BoxImpl for EpicCreateAssetProject {}
}

glib::wrapper! {
    pub struct EpicCreateAssetProject(ObjectSubclass<imp::EpicCreateAssetProject>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicCreateAssetProject {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicCreateAssetProject {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_target_directories(&self) {
        let self_ = self.imp();
        self_.select_target_directory.remove_all();
        for dir in self_.settings.strv("unreal-projects-directories") {
            self_.select_target_directory.append(Some(&dir), &dir);
            if let None = self_.select_target_directory.active_text() {
                self_.select_target_directory.set_active_id(Some(&dir));
            }
        }
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

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("create_asset_project", Some(actions));

        action!(
            actions,
            "create",
            clone!(@weak self as cap => move |_, _| {
                cap.create();
            })
        );
    }

    fn create(&self) {
        let self_ = self.imp();
        if let Some(dm) = self_.download_manager.get() {
            if let Some(asset_info) = &*self_.asset.borrow() {
                dm.add_asset_download(self.selected_version(), asset_info.clone());
                self.emit_by_name::<()>("start-download", &[]);
            }
        }
    }

    pub fn set_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_ = self.imp();
        self_.asset.replace(Some(asset.clone()));
    }

    pub fn selected_version(&self) -> String {
        self.property("selected-version")
    }
}
