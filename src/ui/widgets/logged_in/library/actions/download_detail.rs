use crate::ui::widgets::download_manager::asset::Asset;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;

pub mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/download_detail.ui")]
    pub struct EpicDownloadDetails {
        selected_version: RefCell<Option<String>>,
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        pub manifest: RefCell<Option<egs_api::api::types::download_manifest::DownloadManifest>>,
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub settings: gio::Settings,
        #[template_child]
        pub select_target_directory: TemplateChild<gtk4::ComboBoxText>,
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
                manifest: RefCell::new(None),
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

    impl ObjectImpl for EpicDownloadDetails {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            obj.set_target_directories();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder("start-download")
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> =
                Lazy::new(|| vec![glib::ParamSpecString::builder("selected-version").build()]);

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
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

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
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
        glib::Object::new()
    }

    pub fn set_target_directories(&self) {
        let self_ = self.imp();
        self_.select_target_directory.remove_all();
        for dir in self_.settings.strv("unreal-vault-directories") {
            self_.select_target_directory.append(
                Some(&dir),
                &format!(
                    "{}{}",
                    dir,
                    if self_.select_target_directory.active_text().is_none() {
                        " (default)"
                    } else {
                        ""
                    }
                ),
            );
            if self_.select_target_directory.active_text().is_none() {
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
        self.insert_action_group("download_details", Some(actions));

        action!(
            actions,
            "download_all",
            clone!(
                #[weak(rename_to=download_details)]
                self,
                move |_, _| {
                    download_details.download_all();
                }
            )
        );
    }

    fn download_all(&self) {
        let self_ = self.imp();
        if let Some(dm) = self_.download_manager.get() {
            if let Some(asset_info) = &*self_.asset.borrow() {
                dm.add_asset_download(
                    self.selected_version(),
                    asset_info.clone(),
                    &self_
                        .select_target_directory
                        .active_id()
                        .map(|v| v.to_string()),
                    None,
                );
                self.emit_by_name::<()>("start-download", &[]);
            }
        }
    }

    pub fn set_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_ = self.imp();
        if let Some(asset_info) = &*self_.asset.borrow() {
            // Remove old manifest if we are setting a new asset
            if !asset_info.id.eq(&asset.id) {
                self_.manifest.replace(None);
            }
        };
        self_.asset.replace(Some(asset.clone()));
        self.set_target_directories();
    }

    pub fn set_manifest(
        &self,
        manifest: &egs_api::api::types::download_manifest::DownloadManifest,
    ) {
        let self_ = self.imp();
        self_.manifest.replace(Some(manifest.clone()));
    }

    pub fn selected_version(&self) -> String {
        self.property("selected-version")
    }
}
