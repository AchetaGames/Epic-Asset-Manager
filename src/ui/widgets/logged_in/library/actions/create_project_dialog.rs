use crate::ui::widgets::download_manager::asset::Asset;
use crate::ui::widgets::download_manager::EpicDownloadManager;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk4::glib::clone;
use gtk4::{self, gio, StringList};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;
use log::{debug, info};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;
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
        pub install_location_combo: TemplateChild<gtk4::DropDown>,
        #[template_child]
        pub browse_location_button: TemplateChild<gtk4::Button>,
        #[template_child]
        pub engine_version_combo: TemplateChild<gtk4::DropDown>,
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
                browse_location_button: TemplateChild::default(),
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

        // Browse button click
        self_.browse_location_button.connect_clicked(clone!(
            #[weak(rename_to=dialog)]
            self,
            move |_| {
                dialog.browse_for_location();
            }
        ));
    }

    fn browse_for_location(&self) {
        let file_dialog = gtk4::FileChooserDialog::new(
            Some("Select Project Directory"),
            Some(self),
            gtk4::FileChooserAction::SelectFolder,
            &[
                ("Select", gtk4::ResponseType::Accept),
                ("Cancel", gtk4::ResponseType::Cancel),
            ],
        );
        file_dialog.set_modal(true);
        file_dialog.set_transient_for(Some(self));

        file_dialog.connect_response(clone!(
            #[weak(rename_to=dialog)]
            self,
            move |d, response| {
                if response == gtk4::ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            dialog.add_location(path.to_str().unwrap());
                        }
                    }
                }
                d.destroy();
            }
        ));

        file_dialog.show();
    }

    fn add_location(&self, path: &str) {
        let self_ = self.imp();
        if let Some(model) = &*self_.locations_model.borrow() {
            // Check if already exists
            for i in 0..model.n_items() {
                if let Some(existing) = model.string(i) {
                    if existing.as_str() == path {
                        // Select existing
                        self_.install_location_combo.set_selected(i);
                        return;
                    }
                }
            }
            // Add new and select it
            let pos = model.n_items();
            model.append(path);
            self_.install_location_combo.set_selected(pos);

            // Persist to settings
            let mut current = self_.settings.strv("unreal-projects-directories");
            if !current.contains(&gtk4::glib::GString::from(path)) {
                current.push(gtk4::glib::GString::from(path.to_string()));
                let _ = self_.settings.set_strv("unreal-projects-directories", current);
            }
        }
        self.validate_target_directory();
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
        let mut found_versions: Vec<String> = Vec::new();

        // 1. Read from Install.ini (Epic Games Launcher installations)
        for (_guid, path) in Self::read_engines_ini() {
            if let Some(version) = crate::models::engine_data::EngineData::read_engine_version(&path) {
                let version_str = version.format();
                if !version_str.is_empty() && !found_versions.contains(&version_str) {
                    found_versions.push(version_str);
                }
            }
        }

        // 2. Scan configured engine directories
        for dir in self_.settings.strv("unreal-engine-directories") {
            self.scan_engine_directory(&dir.to_string(), &mut found_versions);
        }

        // Sort versions descending (newest first)
        found_versions.sort_by(|a, b| {
            let parse_version = |s: &str| -> (i32, i32, i32) {
                let parts: Vec<i32> = s.split('.').filter_map(|p| p.parse().ok()).collect();
                (
                    *parts.first().unwrap_or(&0),
                    *parts.get(1).unwrap_or(&0),
                    *parts.get(2).unwrap_or(&0),
                )
            };
            parse_version(b).cmp(&parse_version(a))
        });

        // Add to model
        for version in &found_versions {
            model.append(version);
        }

        // Fallback if no engines found
        if model.n_items() == 0 {
            model.append("No engines found");
            self_.create_button.set_sensitive(false);
        }

        info!("Found {} engine versions: {:?}", found_versions.len(), found_versions);

        self_.engine_version_combo.set_model(Some(&model));
        self_.engines_model.replace(Some(model));

        // Select first item
        if self_.engine_version_combo.model().is_some() {
            self_.engine_version_combo.set_selected(0);
        }
    }

    fn read_engines_ini() -> HashMap<String, String> {
        let ini = gtk4::glib::KeyFile::new();
        let mut dir = gtk4::glib::home_dir();
        dir.push(".config");
        dir.push("Epic");
        dir.push("UnrealEngine");
        dir.push("Install.ini");

        let mut result: HashMap<String, String> = HashMap::new();
        if ini.load_from_file(&dir, gtk4::glib::KeyFileFlags::NONE).is_err() {
            return result;
        }

        if let Ok(keys) = ini.keys("Installations") {
            for item in keys {
                if let Ok(path) = ini.value("Installations", item.as_str()) {
                    let guid: String = item
                        .to_string()
                        .chars()
                        .filter(|c| c != &'{' && c != &'}')
                        .collect();
                    match path.to_string().strip_suffix('/') {
                        None => {
                            result.insert(guid, path.to_string());
                        }
                        Some(pa) => {
                            result.insert(guid, pa.to_string());
                        }
                    }
                }
            }
        }
        result
    }

    fn scan_engine_directory(&self, dir: &str, found_versions: &mut Vec<String>) {
        let path = PathBuf::from(dir);

        // Check if directory itself is an engine
        if let Some(version) = crate::models::engine_data::EngineData::read_engine_version(dir) {
            let version_str = version.format();
            if !version_str.is_empty() && !found_versions.contains(&version_str) {
                found_versions.push(version_str);
            }
            return;
        }

        // Otherwise scan subdirectories
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    if let Some(version) = crate::models::engine_data::EngineData::read_engine_version(
                        entry_path.to_str().unwrap(),
                    ) {
                        let version_str = version.format();
                        if !version_str.is_empty() && !found_versions.contains(&version_str) {
                            found_versions.push(version_str);
                        }
                    }
                }
            }
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

        let overwrite = self_.overwrite_check.is_active();

        if let Some(asset_info) = &*self_.asset.borrow() {
            // Check if asset is already downloaded locally by checking each release's app_id
            let vaults = self_.settings.strv("unreal-vault-directories");

            if let Some(release_infos) = &asset_info.release_info {
                for ri in release_infos {
                    if let Some(app_id) = &ri.app_id {
                        let locations = crate::models::asset_data::AssetData::downloaded_locations(&vaults, app_id);
                        if let Some(source_path) = locations.first() {
                            // Asset is already downloaded - copy directly
                            info!("Asset already downloaded at {:?}, copying to {:?}", source_path, target_path);

                            let source = source_path.clone();
                            let target = target_path.clone();

                            // Perform copy in background thread and launch project when done
                            std::thread::spawn(move || {
                                if let Err(e) = Self::copy_directory(&source, &target, overwrite) {
                                    log::error!("Failed to copy project: {:?}", e);
                                } else {
                                    log::info!("Project created successfully at {:?}", target);
                                    // Find and launch the .uproject file
                                    Self::launch_project(&target);
                                }
                            });

                            self.emit_by_name::<()>("project-created", &[]);
                            self.close();
                            return;
                        }
                    }
                }
            }

            // Asset not downloaded - use download manager
            if let Some(dm) = self_.download_manager.get() {
                if let Some(version) = &*self_.selected_version.borrow() {
                    debug!("Asset not downloaded, adding to download queue with version: {}", version);
                    dm.add_asset_download(
                        version.clone(),
                        asset_info.clone(),
                        &None,
                        Some(vec![
                            crate::ui::widgets::download_manager::PostDownloadAction::Copy(
                                target_path.to_str().unwrap().to_string(),
                                overwrite,
                            ),
                        ]),
                    );

                    self.emit_by_name::<()>("project-created", &[]);
                    self.close();
                }
            }
        }
    }

    fn copy_directory(source: &PathBuf, target: &PathBuf, overwrite: bool) -> std::io::Result<()> {
        if target.exists() {
            if overwrite {
                std::fs::remove_dir_all(target)?;
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "Target directory already exists",
                ));
            }
        }

        std::fs::create_dir_all(target)?;

        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let source_path = entry.path();
            let file_name = entry.file_name();
            let target_path = target.join(&file_name);

            if file_type.is_dir() {
                Self::copy_directory(&source_path, &target_path, overwrite)?;
            } else {
                std::fs::copy(&source_path, &target_path)?;
            }
        }

        Ok(())
    }

    fn launch_project(project_dir: &PathBuf) {
        // Find .uproject file in the directory
        if let Ok(entries) = std::fs::read_dir(project_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "uproject" {
                        log::info!("Launching project: {:?}", path);
                        // Use gio to open with default application
                        if let Err(e) = std::process::Command::new("gio")
                            .arg("open")
                            .arg(&path)
                            .spawn()
                        {
                            log::error!("Failed to launch project with gio: {:?}", e);
                            // Fallback to xdg-open
                            if let Err(e2) = std::process::Command::new("xdg-open")
                                .arg(&path)
                                .spawn()
                            {
                                log::error!("Failed to launch project with xdg-open: {:?}", e2);
                            }
                        }
                        return;
                    }
                }
            }
        }
        log::warn!("No .uproject file found in {:?}", project_dir);
    }
}
