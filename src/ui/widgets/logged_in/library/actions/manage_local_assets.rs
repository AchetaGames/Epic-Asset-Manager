use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use log::error;
use std::path::PathBuf;
use std::str::FromStr;

pub mod imp {
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
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder("removed")
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
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
        glib::Object::new()
    }

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
                            row.connect_local(
                                "delete",
                                false,
                                clone!(
                                    #[weak(rename_to=mla)]
                                    self,
                                    #[weak]
                                    row,
                                    #[upgrade_or]
                                    None,
                                    move |_| {
                                        mla.delete(&row);
                                        None
                                    }
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn delete(&self, widget: &super::local_asset::EpicLocalAsset) {
        let self_ = self.imp();
        remove_from_list_box(&self_.local_list, widget);
        remove_from_list_box(&self_.local_list_other, widget);
        if let Some(p) = widget.path() {
            if let Ok(path) = PathBuf::from_str(&p) {
                if path.exists() {
                    if let Some(parent) = path.parent() {
                        if let Err(e) = std::fs::remove_dir_all(parent) {
                            error!("Unable to remove vault data: {:?}", e);
                        };
                    }
                }
            }
        };
        self.emit_by_name::<()>("removed", &[]);
    }

    pub fn empty(&self) -> bool {
        let self_ = self.imp();
        self_.local_list.first_child().is_none() && self_.local_list_other.first_child().is_none()
    }
}

fn remove_from_list_box(list: &gtk4::ListBox, widget: &impl IsA<gtk4::Widget>) {
    if let Some(mut child) = list.first_child() {
        loop {
            let row = child.clone().downcast::<gtk4::ListBoxRow>().unwrap();
            if let Some(i) = row.first_child() {
                if i.eq(widget) {
                    list.remove(&row);
                    break;
                }
            }
            if let Some(next_child) = row.next_sibling() {
                child = next_child;
            } else {
                break;
            }
        }
    }
}
