use crate::ui::widgets::download_manager::asset::Asset;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;
use log::debug;
use std::path::PathBuf;
use std::str::FromStr;

pub mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/create_asset_project.ui")]
    pub struct EpicCreateAssetProject {
        selected_version: RefCell<Option<String>>,
        project_name: RefCell<Option<String>>,
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        pub manifest: RefCell<Option<egs_api::api::types::download_manifest::DownloadManifest>>,
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub settings: gio::Settings,
        #[template_child]
        pub select_target_directory: TemplateChild<gtk4::ComboBoxText>,
        #[template_child]
        pub warning_row: TemplateChild<gtk4::InfoBar>,
        #[template_child]
        pub overwrite: TemplateChild<gtk4::CheckButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicCreateAssetProject {
        const NAME: &'static str = "EpicCreateAssetProject";
        type Type = super::EpicCreateAssetProject;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                selected_version: RefCell::new(None),
                project_name: RefCell::new(None),
                asset: RefCell::new(None),
                manifest: RefCell::new(None),
                actions: gio::SimpleActionGroup::new(),
                download_manager: OnceCell::new(),
                settings: gio::Settings::new(crate::config::APP_ID),
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

    impl ObjectImpl for EpicCreateAssetProject {
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
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("selected-version").build(),
                    glib::ParamSpecString::builder("project-name").build(),
                ]
            });

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
                "project-name" => {
                    let project_name = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.project_name.replace(project_name);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "selected-version" => self.selected_version.borrow().to_value(),
                "project-name" => self.project_name.borrow().to_value(),
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
        glib::Object::new()
    }

    pub fn set_target_directories(&self) {
        let self_ = self.imp();
        self_.select_target_directory.remove_all();
        for dir in self_.settings.strv("unreal-projects-directories") {
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
        debug!("Set Download manager");

        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("create_asset_project", Some(actions));

        action!(
            actions,
            "create",
            clone!(
                #[weak(rename_to=cap)]
                self,
                move |_, _| {
                    cap.create();
                }
            )
        );

        self_.select_target_directory.connect_changed(clone!(
            #[weak(rename_to=cap)]
            self,
            move |_| {
                cap.directory_changed();
            }
        ));
    }

    fn directory_changed(&self) {
        self.validate_target_directory();
    }

    fn validate_target_directory(&self) {
        let self_ = self.imp();
        if let Some(project) = self.project_name() {
            if let Some(id) = self_.select_target_directory.active_id() {
                let mut path = PathBuf::from_str(id.as_str()).unwrap();
                path.push(project);
                if path.exists() {
                    self_.warning_row.set_visible(true);
                }
            }
        }
    }

    fn create(&self) {
        let self_ = self.imp();
        if let Some(dm) = self_.download_manager.get() {
            if let Some(asset_info) = &*self_.asset.borrow() {
                if let Some(project) = self.project_name() {
                    if let Some(id) = self_.select_target_directory.active_id() {
                        let mut path = PathBuf::from_str(id.as_str()).unwrap();
                        path.push(project);
                        dm.add_asset_download(
                            self.selected_version(),
                            asset_info.clone(),
                            &None,
                            Some(vec![
                                crate::ui::widgets::download_manager::PostDownloadAction::Copy(
                                    path.to_str().unwrap().to_string(),
                                    self_.overwrite.is_active(),
                                ),
                            ]),
                        );
                        self.emit_by_name::<()>("start-download", &[]);
                    }
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
    }

    pub fn set_manifest(
        &self,
        manifest: &egs_api::api::types::download_manifest::DownloadManifest,
    ) {
        let self_ = self.imp();
        self_.manifest.replace(Some(manifest.clone()));
        self.process_manifest();
    }

    fn process_manifest(&self) {
        let self_ = self.imp();
        if let Some(manifest) = &*self_.manifest.borrow() {
            for file in manifest.files().keys() {
                if file.ends_with(".uproject") {
                    self.set_property("project-name", file[..file.len() - 9].to_string());
                    self.validate_target_directory();
                    break;
                }
            }
        }
    }

    pub fn selected_version(&self) -> String {
        self.property("selected-version")
    }

    pub fn project_name(&self) -> Option<String> {
        self.property("project-name")
    }
}
