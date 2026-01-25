use crate::ui::widgets::download_manager::asset::Asset;
use crate::ui::widgets::download_manager::EpicDownloadManager;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk4::glib::clone;
use gtk4::{self, gio, StringList};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;
use log::debug;
use std::cell::RefCell;
use std::path::PathBuf;

pub mod imp {
    use super::*;
    use once_cell::sync::OnceCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/create_project_dialog.ui")]
    pub struct EpicCreateProjectDialog {
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        pub manifest: RefCell<Option<egs_api::api::types::download_manifest::DownloadManifest>>,
        pub selected_version: RefCell<Option<String>>,
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub settings: gio::Settings,
        pub locations_model: RefCell<Option<StringList>>,
        pub engines_model: RefCell<Option<StringList>>,
        #[template_child]
        pub project_name_entry: TemplateChild<gtk4::Entry>,
        #[template_child]
        pub install_location_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub engine_version_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub warning_bar: TemplateChild<gtk4::InfoBar>,
        #[template_child]
        pub warning_label: TemplateChild<gtk4::Label>,
        #[template_child]
        pub overwrite_check: TemplateChild<gtk4::CheckButton>,
        #[template_child]
        pub create_button: TemplateChild<gtk4::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicCreateProjectDialog {
        const NAME: &'static str = "EpicCreateProjectDialog";
        type Type = super::EpicCreateProjectDialog;
        type ParentType = adw::Window;

        fn new() -> Self {
            Self {
                asset: RefCell::new(None),
                manifest: RefCell::new(None),
                selected_version: RefCell::new(None),
                actions: gio::SimpleActionGroup::new(),
                download_manager: OnceCell::new(),
                settings: gio::Settings::new(crate::config::APP_ID),
                locations_model: RefCell::new(None),
                engines_model: RefCell::new(None),
                project_name_entry: TemplateChild::default(),
                install_location_combo: TemplateChild::default(),
                engine_version_combo: TemplateChild::default(),
                warning_bar: TemplateChild::default(),
                warning_label: TemplateChild::default(),
                overwrite_check: TemplateChild::default(),
                create_button: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EpicCreateProjectDialog {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            obj.setup_locations();
            obj.setup_engines();
            obj.setup_events();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder("project-created")
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for EpicCreateProjectDialog {}
    impl WindowImpl for EpicCreateProjectDialog {}
    impl AdwWindowImpl for EpicCreateProjectDialog {}
}

glib::wrapper! {
    pub struct EpicCreateProjectDialog(ObjectSubclass<imp::EpicCreateProjectDialog>)
        @extends gtk4::Widget, gtk4::Window, adw::Window,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl Default for EpicCreateProjectDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicCreateProjectDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_download_manager(&self, dm: &EpicDownloadManager) {
        let self_ = self.imp();
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn set_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_ = self.imp();
        self_.asset.replace(Some(asset.clone()));

        // Set default project name from asset title
        if let Some(title) = &asset.title {
            // Clean up the title for use as a project name
            let clean_name = title
                .replace(" ", "_")
                .replace("-", "_")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>();
            self_.project_name_entry.set_text(&clean_name);
        }
    }

    pub fn set_manifest(
        &self,
        manifest: &egs_api::api::types::download_manifest::DownloadManifest,
    ) {
        let self_ = self.imp();
        self_.manifest.replace(Some(manifest.clone()));

        // Try to extract project name from manifest (.uproject file)
        for file in manifest.files().keys() {
            if file.ends_with(".uproject") {
                let project_name = &file[..file.len() - 9];
                self_.project_name_entry.set_text(project_name);
                break;
            }
        }

        self.validate_target_directory();
    }

    pub fn set_selected_version(&self, version: &str) {
        let self_ = self.imp();
        self_.selected_version.replace(Some(version.to_string()));
    }

    fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("create_project", Some(actions));

        action!(
            actions,
            "create",
            clone!(
                #[weak(rename_to=dialog)]
                self,
                move |_, _| {
                    dialog.create_project();
                }
            )
        );
    }

    fn setup_events(&self) {
        let self_ = self.imp();

        // Validate when project name changes
        self_.project_name_entry.connect_changed(clone!(
            #[weak(rename_to=dialog)]
            self,
            move |_| {
                dialog.validate_target_directory();
            }
        ));

        // Validate when location changes
        self_.install_location_combo.connect_selected_notify(clone!(
            #[weak(rename_to=dialog)]
            self,
            move |_| {
                dialog.validate_target_directory();
            }
        ));
    }

    fn setup_locations(&self) {
        let self_ = self.imp();
        let model = StringList::new(&[]);

        for dir in self_.settings.strv("unreal-projects-directories") {
            model.append(&dir);
        }

        self_.install_location_combo.set_model(Some(&model));
        self_.locations_model.replace(Some(model));

        // Select first item
        if self_.install_location_combo.model().is_some() {
            self_.install_location_combo.set_selected(0);
        }
    }

    fn setup_engines(&self) {
        let self_ = self.imp();
        let model = StringList::new(&[]);

        // Get engine directories and find installed versions
        for dir in self_.settings.strv("unreal-engine-directories") {
            let path = PathBuf::from(dir.as_str());
            if path.exists() {
                if let Ok(entries) = std::fs::read_dir(&path) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            // Check if this looks like an engine directory
                            let version_file = entry_path.join("Engine/Build/Build.version");
                            if version_file.exists() {
                                if let Some(name) = entry_path.file_name() {
                                    model.append(&name.to_string_lossy());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add some default versions if none found
        if model.n_items() == 0 {
            model.append("5.4");
            model.append("5.3");
            model.append("5.2");
        }

        self_.engine_version_combo.set_model(Some(&model));
        self_.engines_model.replace(Some(model));

        // Select first item
        if self_.engine_version_combo.model().is_some() {
            self_.engine_version_combo.set_selected(0);
        }
    }

    fn validate_target_directory(&self) {
        let self_ = self.imp();
        let project_name = self_.project_name_entry.text();

        if project_name.is_empty() {
            self_.warning_bar.set_visible(false);
            self_.create_button.set_sensitive(false);
            return;
        }

        self_.create_button.set_sensitive(true);

        if let Some(model) = &*self_.locations_model.borrow() {
            let selected = self_.install_location_combo.selected();
            if let Some(location) = model.string(selected) {
                let mut path = PathBuf::from(location.as_str());
                path.push(project_name.as_str());

                if path.exists() {
                    self_.warning_bar.set_visible(true);
                    self_.warning_label.set_label("Project already exists in the target directory");
                } else {
                    self_.warning_bar.set_visible(false);
                }
            }
        }
    }

    fn get_selected_location(&self) -> Option<String> {
        let self_ = self.imp();
        if let Some(model) = &*self_.locations_model.borrow() {
            let selected = self_.install_location_combo.selected();
            model.string(selected).map(|s| s.to_string())
        } else {
            None
        }
    }

    fn get_selected_engine(&self) -> Option<String> {
        let self_ = self.imp();
        if let Some(model) = &*self_.engines_model.borrow() {
            let selected = self_.engine_version_combo.selected();
            model.string(selected).map(|s| s.to_string())
        } else {
            None
        }
    }

    fn create_project(&self) {
        let self_ = self.imp();

        let project_name = self_.project_name_entry.text().to_string();
        if project_name.is_empty() {
            return;
        }

        let Some(location) = self.get_selected_location() else {
            return;
        };

        let mut target_path = PathBuf::from(&location);
        target_path.push(&project_name);

        // Check if project exists and overwrite is not checked
        if target_path.exists() && !self_.overwrite_check.is_active() {
            self_.warning_bar.set_visible(true);
            return;
        }

        debug!("Creating project: {} at {:?}", project_name, target_path);

        if let Some(dm) = self_.download_manager.get() {
            if let Some(asset_info) = &*self_.asset.borrow() {
                if let Some(version) = &*self_.selected_version.borrow() {
                    dm.add_asset_download(
                        version.clone(),
                        asset_info.clone(),
                        &None,
                        Some(vec![
                            crate::ui::widgets::download_manager::PostDownloadAction::Copy(
                                target_path.to_str().unwrap().to_string(),
                                self_.overwrite_check.is_active(),
                            ),
                        ]),
                    );

                    self.emit_by_name::<()>("project-created", &[]);
                    self.close();
                }
            }
        }
    }
}
