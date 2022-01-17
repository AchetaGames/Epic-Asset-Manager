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
    #[template(resource = "/io/github/achetagames/epic_asset_manager/download_detail.ui")]
    pub struct EpicDownloadDetails {
        selected_version: RefCell<Option<String>>,
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicDownloadDetails {
        const NAME: &'static str = "EpicDownloadDetails";
        type Type = super::EpicDownloadDetails;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                selected_version: RefCell::new(None),
                asset: RefCell::new(None),
                actions: gio::SimpleActionGroup::new(),
                download_manager: OnceCell::new(),
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

    impl ObjectImpl for EpicDownloadDetails {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
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

    impl WidgetImpl for EpicDownloadDetails {}
    impl BoxImpl for EpicDownloadDetails {}
}

glib::wrapper! {
    pub struct EpicDownloadDetails(ObjectSubclass<imp::EpicDownloadDetails>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicDownloadDetails {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicDownloadDetails {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_: &imp::EpicDownloadDetails = imp::EpicDownloadDetails::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }

        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn setup_actions(&self) {
        let self_: &imp::EpicDownloadDetails = imp::EpicDownloadDetails::from_instance(self);
        let actions = &self_.actions;
        self.insert_action_group("download_details", Some(actions));

        action!(
            actions,
            "download_all",
            clone!(@weak self as download_details => move |_, _| {
                let self_: &imp::EpicDownloadDetails = imp::EpicDownloadDetails::from_instance(&download_details);
                if let Some(dm) = self_.download_manager.get() {
                    if let Some(asset_info) = &*self_.asset.borrow() {
                        dm.add_asset_download(download_details.selected_version(), asset_info.clone());
                        download_details.emit_by_name::<()>("start-download", &[]);
                    }
                }
            })
        );
    }

    pub fn set_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_: &imp::EpicDownloadDetails = imp::EpicDownloadDetails::from_instance(self);
        self_.asset.replace(Some(asset.clone()));
    }

    pub fn selected_version(&self) -> String {
        let value: glib::Value = self.property("selected-version");
        if let Ok(id_opt) = value.get::<String>() {
            return id_opt;
        }
        "".to_string()
    }
}
