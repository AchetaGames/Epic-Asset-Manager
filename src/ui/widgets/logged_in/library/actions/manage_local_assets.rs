use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub(crate) mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::window::EpicAssetManagerWindow;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/manage_local_assets.ui")]
    pub struct EpicLocalAssets {
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub settings: gtk4::gio::Settings,
        #[template_child]
        pub local_list: TemplateChild<gtk4::ListBox>,
        #[template_child]
        pub other_expander: TemplateChild<gtk4::Expander>,
        #[template_child]
        pub local_list_other: TemplateChild<gtk4::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicLocalAssets {
        const NAME: &'static str = "EpicLocalAssets";
        type Type = super::EpicLocalAssets;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                asset: RefCell::new(None),
                actions: gio::SimpleActionGroup::new(),
                download_manager: OnceCell::new(),
                window: OnceCell::new(),
                local_list: TemplateChild::default(),
                other_expander: TemplateChild::default(),
                settings: gio::Settings::new(crate::config::APP_ID),
                local_list_other: TemplateChild::default(),
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

    impl ObjectImpl for EpicLocalAssets {
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
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| vec![]);

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            _value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicLocalAssets {}
    impl BoxImpl for EpicLocalAssets {}
}

glib::wrapper! {
    pub struct EpicLocalAssets(ObjectSubclass<imp::EpicLocalAssets>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicLocalAssets {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicLocalAssets {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn setup_actions(&self) {}

    pub fn set_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_ = self.imp();
        self_.asset.replace(Some(asset.clone()));
    }

    pub fn update_local_versions(&self, release: &str) {
        let self_ = self.imp();
        let vaults = self_.settings.strv("unreal-vault-directories");
        while let Some(el) = self_.local_list.first_child() {
            self_.local_list.remove(&el);
        }
        while let Some(el) = self_.local_list_other.first_child() {
            self_.local_list_other.remove(&el);
        }
        self_.other_expander.set_expanded(false);
        self_.other_expander.set_visible(false);

        if let Some(asset) = &*self_.asset.borrow() {
            if let Some(releases) = &asset.release_info {
                for rel in releases {
                    if let Some(app) = &rel.app_id {
                        for location in crate::models::asset_data::AssetData::downloaded_locations(
                            &vaults,
                            app.as_str(),
                        ) {
                            let row = super::local_asset::EpicLocalAsset::new();
                            row.set_property(
                                "label",
                                location
                                    .into_os_string()
                                    .to_str()
                                    .unwrap_or_default()
                                    .to_string(),
                            );
                            if app.eq(release) {
                                &self_.local_list
                            } else {
                                self_.other_expander.set_visible(true);
                                &self_.local_list_other
                            }
                            .append(&row);
                        }
                    }
                }
            }
        }
    }
}
