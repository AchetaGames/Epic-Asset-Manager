use crate::ui::widgets::download_manager::asset::Asset;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use std::str::FromStr;

pub mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::window::EpicAssetManagerWindow;
    use adw::gtk;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/add_to_project.ui")]
    pub struct EpicAddToProject {
        selected_version: RefCell<Option<String>>,
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        pub manifest: RefCell<Option<egs_api::api::types::download_manifest::DownloadManifest>>,
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        #[template_child]
        pub select_target_directory: TemplateChild<gtk4::ComboBoxText>,
        #[template_child]
        pub warning_row: TemplateChild<gtk::InfoBar>,
        #[template_child]
        pub overwrite: TemplateChild<gtk4::CheckButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicAddToProject {
        const NAME: &'static str = "EpicAddToProject";
        type Type = super::EpicAddToProject;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                selected_version: RefCell::new(None),
                asset: RefCell::new(None),
                manifest: RefCell::new(None),
                actions: gio::SimpleActionGroup::new(),
                download_manager: OnceCell::new(),
                window: OnceCell::new(),
                select_target_directory: TemplateChild::default(),
                warning_row: TemplateChild::default(),
                overwrite: TemplateChild::default(),
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

    impl ObjectImpl for EpicAddToProject {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_actions();
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

    impl WidgetImpl for EpicAddToProject {}
    impl BoxImpl for EpicAddToProject {}
}

glib::wrapper! {
    pub struct EpicAddToProject(ObjectSubclass<imp::EpicAddToProject>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicAddToProject {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicAddToProject {
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

    pub fn set_target_directories(&self) {
        let self_ = self.imp();
        self_.select_target_directory.remove_all();
        get_action!(self_.actions, @download_all).set_enabled(false);
        if let Some(w) = self_.window.get() {
            let w_ = w.imp();
            let l = w_.logged_in_stack.clone();
            let l_ = l.imp();
            let p = l_.projects.imp();
            for path in p.projects.borrow().keys() {
                self_.select_target_directory.append(Some(path), path);
            }
        }
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
                    download_details.add_to_project();
                }
            )
        );

        self_.select_target_directory.connect_changed(clone!(
            #[weak(rename_to=atp)]
            self,
            move |_| {
                atp.directory_changed();
            }
        ));
    }

    fn add_to_project(&self) {
        let self_ = self.imp();
        if let Some(dm) = self_.download_manager.get() {
            if let Some(asset_info) = &*self_.asset.borrow() {
                if let Some(id) = self_.select_target_directory.active_id() {
                    dm.add_asset_download(
                        self.selected_version(),
                        asset_info.clone(),
                        &None,
                        Some(vec![
                            crate::ui::widgets::download_manager::PostDownloadAction::Copy(
                                id.to_string(),
                                self_.overwrite.is_active(),
                            ),
                        ]),
                    );
                    self.emit_by_name::<()>("start-download", &[]);
                }
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
        self_.warning_row.set_visible(false);
        self_.asset.replace(Some(asset.clone()));
        self.set_target_directories();
    }

    fn validate_target_directory(&self) {
        let self_ = self.imp();
        self_
            .warning_row
            .set_tooltip_text(Some("Files that would be overwritten: "));
        if let Some(manifest) = &*self_.manifest.borrow() {
            if let Some(id) = self_.select_target_directory.active_id() {
                let path = std::path::PathBuf::from_str(id.as_str()).unwrap();
                for (file, _) in manifest.files() {
                    let mut p = path.clone();
                    p.push(file);
                    if p.exists() {
                        self_.warning_row.set_visible(true);
                        if let Some(old) = self_.warning_row.tooltip_text() {
                            self_.warning_row.set_tooltip_text(Some(&format!(
                                "{}\n{}",
                                old,
                                p.to_str().unwrap_or_default()
                            )));
                        };
                    }
                }
            }
        }
    }

    fn directory_changed(&self) {
        let self_ = self.imp();
        get_action!(self_.actions, @download_all).set_enabled(true);
        self.validate_target_directory();
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
