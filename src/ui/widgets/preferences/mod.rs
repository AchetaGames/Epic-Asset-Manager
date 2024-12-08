pub mod dir_row;

use adw::prelude::PreferencesWindowExt;
use gtk4::gio::{File, FileQueryInfoFlags, FileType, SettingsBindFlags};
use gtk4::glib::clone;
use gtk4::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::action;
use log::{debug, error};
use once_cell::sync::OnceCell;
use std::collections::HashMap;

pub mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use adw::subclass::{preferences_window::PreferencesWindowImpl, window::AdwWindowImpl};
    use glib::subclass::{self};
    use std::cell::RefCell;

    #[derive(CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/preferences.ui")]
    pub struct PreferencesWindow {
        pub settings: gio::Settings,
        pub actions: gio::SimpleActionGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub directory_rows: RefCell<
            HashMap<
                super::DirectoryConfigType,
                Vec<(
                    String,
                    crate::ui::widgets::preferences::dir_row::DirectoryRow,
                )>,
            >,
        >,
        pub file_chooser: RefCell<Option<gtk4::FileChooserDialog>>,
        #[template_child]
        pub cache_directory_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub temp_directory_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub dark_theme_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub unreal_engine_project_directories_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub unreal_engine_vault_directories_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub unreal_engine_directories_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub github_token: TemplateChild<gtk4::PasswordEntry>,
        #[template_child]
        pub github_user: TemplateChild<gtk4::Entry>,
        #[template_child]
        pub dark_theme_switch: TemplateChild<gtk4::Switch>,
        #[template_child]
        pub sidebar_switch: TemplateChild<gtk4::Switch>,
        #[template_child]
        pub default_view_selection: TemplateChild<gtk4::ComboBoxText>,
        #[template_child]
        pub log_level_selection: TemplateChild<gtk4::ComboBoxText>,
        #[template_child]
        pub default_category_selection: TemplateChild<gtk4::ComboBoxText>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesWindow {
        const NAME: &'static str = "PreferencesWindow";
        type Type = super::PreferencesWindow;
        type ParentType = adw::PreferencesWindow;

        fn new() -> Self {
            let settings = gio::Settings::new(crate::config::APP_ID);

            Self {
                settings,
                actions: gio::SimpleActionGroup::new(),
                window: OnceCell::new(),
                directory_rows: RefCell::new(HashMap::new()),
                file_chooser: RefCell::new(None),
                cache_directory_row: TemplateChild::default(),
                temp_directory_row: TemplateChild::default(),
                dark_theme_group: TemplateChild::default(),
                unreal_engine_project_directories_box: TemplateChild::default(),
                unreal_engine_vault_directories_box: TemplateChild::default(),
                unreal_engine_directories_box: TemplateChild::default(),
                github_token: TemplateChild::default(),
                github_user: TemplateChild::default(),
                dark_theme_switch: TemplateChild::default(),
                sidebar_switch: TemplateChild::default(),
                default_view_selection: TemplateChild::default(),
                log_level_selection: TemplateChild::default(),
                default_category_selection: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.bind_settings();
            obj.load_settings();
            obj.setup_actions();
        }
    }
    impl WidgetImpl for PreferencesWindow {}
    impl WindowImpl for PreferencesWindow {}
    impl AdwWindowImpl for PreferencesWindow {}
    impl PreferencesWindowImpl for PreferencesWindow {}
}

glib::wrapper! {
    pub struct PreferencesWindow(ObjectSubclass<imp::PreferencesWindow>)
        @extends gtk4::Widget, gtk4::Window, adw::Window, adw::PreferencesWindow;
}

#[derive(PartialEq, Debug, Clone, Copy, Hash, Eq)]
pub enum DirectoryConfigType {
    Cache,
    Temp,
    Vault,
    Engine,
    Projects,
    Games,
}

impl Default for PreferencesWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl PreferencesWindow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        if self_.window.get().is_some() {
            return;
        }
        self_.window.set(window.clone()).unwrap();
        self.load_secrets();
    }

    pub fn bind_settings(&self) {
        let self_ = self.imp();
        self_
            .settings
            .bind("dark-mode", &*self_.dark_theme_switch, "active")
            .build();
        self_
            .settings
            .bind("sidebar-expanded", &*self_.sidebar_switch, "active")
            .build();
        self_
            .settings
            .connect_changed(Some("dark-mode"), |settings, _key| {
                let style_manager = adw::StyleManager::default();
                if settings.boolean("dark-mode") {
                    style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
                } else if !style_manager.system_supports_color_schemes() {
                    style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
                } else {
                    style_manager.set_color_scheme(adw::ColorScheme::Default);
                };
            });
        self_
            .settings
            .bind("cache-directory", &*self_.cache_directory_row, "subtitle")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
        self_
            .settings
            .bind(
                "temporary-download-directory",
                &*self_.temp_directory_row,
                "subtitle",
            )
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        self_
            .settings
            .bind("github-user", &*self_.github_user, "text")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        self_.github_user.connect_changed(clone!(
            #[weak(rename_to=preferences)]
            self,
            move |_| {
                preferences.github_user_changed();
            }
        ));

        self_.default_view_selection.connect_changed(clone!(
            #[weak(rename_to=preferences)]
            self,
            move |_| {
                preferences.default_view_changed();
            }
        ));

        self_.log_level_selection.connect_changed(clone!(
            #[weak(rename_to=preferences)]
            self,
            move |_| {
                preferences.log_level_changed();
            }
        ));

        self_.default_category_selection.connect_changed(clone!(
            #[weak(rename_to=preferences)]
            self,
            move |_| {
                preferences.default_category_changed();
            }
        ));
    }

    fn log_level_changed(&self) {
        let self_ = self.imp();

        if let Ok(level) = self_
            .log_level_selection
            .active_id()
            .unwrap_or_else(|| "0".into())
            .parse::<i32>()
        {
            self_.settings.set_int("log-level", level).unwrap();
            Self::set_log_level(level);
        };
    }

    pub fn set_log_level(level: i32) {
        match level {
            1 => log::set_max_level(log::LevelFilter::Warn),
            2 => log::set_max_level(log::LevelFilter::Info),
            3 => log::set_max_level(log::LevelFilter::Debug),
            4 => log::set_max_level(log::LevelFilter::Trace),
            _ => log::set_max_level(log::LevelFilter::Error),
        }
    }

    fn default_view_changed(&self) {
        let self_ = self.imp();
        self_
            .settings
            .set_string(
                "default-view",
                &self_
                    .default_view_selection
                    .active_id()
                    .unwrap_or_else(|| "library".into()),
            )
            .unwrap();
    }

    fn default_category_changed(&self) {
        let self_ = self.imp();
        self_
            .settings
            .set_string(
                "default-category",
                &self_
                    .default_category_selection
                    .active_id()
                    .unwrap_or_else(|| "unreal".into()),
            )
            .unwrap();
    }

    fn github_user_changed(&self) {
        let self_ = self.imp();
        if let Some(w) = self_.window.get() {
            let win_ = w.imp();
            let model = win_.model.borrow();
            model.validate_registry_login(
                self_.github_user.text().as_str().to_string(),
                self_.github_token.text().as_str().to_string(),
            );
        };
    }

    pub fn load_settings(&self) {
        let self_ = self.imp();
        for dir in self_.settings.strv("unreal-projects-directories") {
            self.add_directory_row(
                &self_.unreal_engine_project_directories_box,
                dir.to_string(),
                DirectoryConfigType::Projects,
            );
        }

        for dir in self_.settings.strv("unreal-vault-directories") {
            self.add_directory_row(
                &self_.unreal_engine_vault_directories_box,
                dir.to_string(),
                DirectoryConfigType::Vault,
            );
        }

        for dir in self_.settings.strv("unreal-engine-directories") {
            self.add_directory_row(
                &self_.unreal_engine_directories_box,
                dir.to_string(),
                DirectoryConfigType::Engine,
            );
        }

        let view = self_.settings.string("default-view");
        self_.default_view_selection.set_active_id(Some(&view));
        let level = self_.settings.int("log-level");
        self_
            .log_level_selection
            .set_active_id(Some(&format!("{level}")));
        self.log_level_changed();
        let category = self_.settings.string("default-category");
        self_
            .default_category_selection
            .set_active_id(Some(&category));
    }

    fn load_secrets(&self) {
        #[cfg(target_os = "linux")]
        {
            let self_ = self.imp();
            if let Some(w) = self_.window.get() {
                let win_ = w.imp();
                #[cfg(target_os = "linux")]
                {
                    win_.model.borrow().secret_service.as_ref().map_or_else(
                        || {
                            // w.add_notification("ss_none", "org.freedesktop.Secret.Service not available for use, secrets will not be stored securely", gtk4::MessageType::Warning);
                            self.load_secrets_insecure();
                        },
                        |ss| {
                            if let Ok(collection) = ss.get_any_collection() {
                                if let Ok(items) = collection.search_items(HashMap::from([(
                                    "application",
                                    crate::config::APP_ID,
                                )])) {
                                    for item in items {
                                        let Ok(label) = item.get_label() else {
                                            debug!("No label skipping");
                                            continue;
                                        };
                                        debug!("Loading: {label}");
                                        if label.as_str() == "eam_github_token" {
                                            if let Ok(d) = item.get_secret() {
                                                if let Ok(s) = std::str::from_utf8(d.as_slice()) {
                                                    self_.github_token.set_text(s);
                                                }
                                            };
                                        }
                                    }
                                };
                            };
                        },
                    );
                }
                #[cfg(target_os = "windows")]
                {
                    self.load_secrets_insecure();
                }
            };
            self_.github_token.connect_changed(clone!(
                #[weak(rename_to=preferences)]
                self,
                move |_| {
                    preferences.github_token_changed();
                }
            ));
        }
    }

    fn load_secrets_insecure(&self) {
        let self_ = self.imp();
        let gh_token = self_.settings.string("github-token");
        if !gh_token.is_empty() {
            self_.github_token.set_text(&gh_token);
        }
    }

    fn save_github_token_insecure(&self) {
        let self_ = self.imp();
        self_
            .settings
            .set_string("github-token", &self_.github_token.text())
            .unwrap();
    }

    fn github_token_changed(&self) {
        let self_ = self.imp();
        if let Some(w) = self_.window.get() {
            let mut attributes = HashMap::new();
            attributes.insert("application", crate::config::APP_ID);
            attributes.insert("type", "token");
            let win_ = w.imp();
            let model = win_.model.borrow();
            #[cfg(target_os = "linux")]
            {
                model.secret_service.as_ref().map_or_else(
                    || {
                        // w.add_notification("ss_none_gh", "org.freedesktop.Secret.Service not available for use, github token will not be saved securely", gtk4::MessageType::Warning);
                        self.save_github_token_insecure();
                    },
                    |ss| {
                        self_.settings.set_string("github-token", "").unwrap();
                        if let Err(e) = ss.get_any_collection().unwrap().create_item(
                            "eam_github_token",
                            attributes.clone(),
                            self_.github_token.text().as_bytes(),
                            true,
                            "text/plain",
                        ) {
                            error!("Failed to save secret {}", e);
                            // w.add_notification("ss_none_gh", "org.freedesktop.Secret.Service not available for use, github token will not be saved securely", gtk4::MessageType::Warning);
                            self.save_github_token_insecure();
                        };
                    },
                );
            }
            #[cfg(target_os = "windows")]
            {
                self.save_github_token_insecure();
            }

            model.validate_registry_login(
                self_.github_user.text().as_str().to_string(),
                self_.github_token.text().as_str().to_string(),
            );
        }
    }

    pub fn switch_to_tab(&self, name: &str) {
        self.set_visible_page_name(name);
    }

    fn handle_file_dialogue_response(
        &self,
        dialog: &gtk4::FileChooserDialog,
        response: gtk4::ResponseType,
        kind: DirectoryConfigType,
    ) {
        if response == gtk4::ResponseType::Accept {
            if let Some(file) = dialog.file() {
                self.set_directory(&file, kind);
            }
        }
        dialog.destroy();
    }

    fn select_directory(&self, title: &'static str, kind: DirectoryConfigType) {
        let dialog: gtk4::FileChooserDialog = self.select_file(&[], title);
        dialog.connect_response(clone!(
            #[weak(rename_to=preferences)]
            self,
            move |d, response| {
                preferences.handle_file_dialogue_response(d, response, kind);
            }
        ));
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;

        self.insert_action_group("preferences", Some(actions));
        action!(
            actions,
            "cache",
            clone!(
                #[weak(rename_to=preferences)]
                self,
                move |_, _| {
                    preferences.select_directory("Cache Directory", DirectoryConfigType::Cache);
                }
            )
        );

        action!(
            actions,
            "temp",
            clone!(
                #[weak(rename_to=preferences)]
                self,
                move |_, _| {
                    preferences.select_directory("Temporary Directory", DirectoryConfigType::Temp);
                }
            )
        );
        action!(
            actions,
            "add_vault",
            clone!(
                #[weak(rename_to=preferences)]
                self,
                move |_, _| {
                    preferences.select_directory("Vault Directory", DirectoryConfigType::Vault);
                }
            )
        );
        action!(
            actions,
            "add_engine",
            clone!(
                #[weak(rename_to=preferences)]
                self,
                move |_, _| {
                    preferences.select_directory("Engine Directory", DirectoryConfigType::Engine);
                }
            )
        );
        action!(
            actions,
            "add_project",
            clone!(
                #[weak(rename_to=preferences)]
                self,
                move |_, _| {
                    preferences
                        .select_directory("Projects Directory", DirectoryConfigType::Projects);
                }
            )
        );
    }

    fn set_directory(&self, dir: &File, kind: DirectoryConfigType) {
        let self_ = self.imp();
        match dir.query_file_type(FileQueryInfoFlags::NONE, gtk4::gio::Cancellable::NONE) {
            FileType::Directory => {
                debug!("Selected Directory");
            }
            _ => {
                return;
            }
        };

        let name = match dir.path() {
            None => return,
            Some(d) => d.into_os_string(),
        };

        match kind {
            DirectoryConfigType::Cache => {
                debug!("Setting the cache directory");
                self_
                    .settings
                    .set_string(
                        "cache-directory",
                        name.as_os_str().to_str().unwrap_or_default(),
                    )
                    .unwrap();
            }
            DirectoryConfigType::Temp => {
                debug!("Setting the temporary directory");
                self_
                    .settings
                    .set_string(
                        "temporary-download-directory",
                        name.as_os_str().to_str().unwrap_or_default(),
                    )
                    .unwrap();
            }
            DirectoryConfigType::Vault
            | DirectoryConfigType::Engine
            | DirectoryConfigType::Projects => {
                if let Some((setting_name, widget)) = self.setting_name_and_box_from_type(kind) {
                    let mut current = self_.settings.strv(setting_name);
                    let Ok(n) = name.into_string() else {
                        error!("Selected directory is not UTF8");
                        return;
                    };
                    if !current.contains(&gtk4::glib::GString::from(n.clone())) {
                        current.push(gtk4::glib::GString::from(n.clone()));
                        self.add_directory_row(widget, n, kind);
                    }
                    self_.settings.set_strv(setting_name, current).unwrap();
                }
            }
            DirectoryConfigType::Games => {}
        };
    }

    fn update_directories(&self, kind: DirectoryConfigType) {
        let self_ = self.imp();
        let rows = self_.directory_rows.borrow();
        if let Some(r) = rows.get(&kind) {
            let v: Vec<&str> = r.iter().map(|i| i.0.as_str()).collect();
            if let Some(setting_name) = Self::setting_name_from_type(kind) {
                self_.settings.set_strv(setting_name, v.as_slice()).unwrap();
            }
        };
    }

    const fn setting_name_from_type(kind: DirectoryConfigType) -> Option<&'static str> {
        match kind {
            DirectoryConfigType::Games | DirectoryConfigType::Cache | DirectoryConfigType::Temp => {
                None
            }
            DirectoryConfigType::Vault => Some("unreal-vault-directories"),
            DirectoryConfigType::Engine => Some("unreal-engine-directories"),
            DirectoryConfigType::Projects => Some("unreal-projects-directories"),
        }
    }

    fn setting_name_and_box_from_type(
        &self,
        kind: DirectoryConfigType,
    ) -> Option<(&'static str, &gtk4::Box)> {
        let self_ = self.imp();
        match kind {
            DirectoryConfigType::Games | DirectoryConfigType::Cache | DirectoryConfigType::Temp => {
                None
            }
            DirectoryConfigType::Vault => Some((
                "unreal-vault-directories",
                &*self_.unreal_engine_vault_directories_box,
            )),
            DirectoryConfigType::Engine => Some((
                "unreal-engine-directories",
                &*self_.unreal_engine_directories_box,
            )),
            DirectoryConfigType::Projects => Some((
                "unreal-projects-directories",
                &*self_.unreal_engine_project_directories_box,
            )),
        }
    }

    fn add_directory_row(&self, target_box: &gtk4::Box, dir: String, kind: DirectoryConfigType) {
        let row: dir_row::DirectoryRow = dir_row::DirectoryRow::new(&dir, self);

        let self_ = self.imp();

        let mut rows = self_.directory_rows.borrow_mut();
        #[allow(clippy::option_if_let_else)]
        match rows.get_mut(&kind) {
            None => {
                row.set_up_enabled(false);
                row.set_down_enabled(false);
                rows.insert(kind, vec![(dir.clone(), row.clone())]);
            }
            Some(r) => {
                r.push((dir.clone(), row.clone()));
                Self::fix_movement_buttons(r);
            }
        };

        let k = kind;
        let dir_c = dir.clone();
        row.connect_local(
            "remove",
            false,
            clone!(
                #[weak(rename_to=preferences)]
                self,
                #[weak]
                target_box,
                #[weak]
                row,
                #[upgrade_or]
                None,
                move |_| {
                    preferences.remove(&target_box, &dir_c, &row, k);
                    None
                }
            ),
        );

        let k = kind;
        let dir_c = dir.clone();
        row.connect_local(
            "move-up",
            false,
            clone!(
                #[weak(rename_to=preferences)]
                self,
                #[weak]
                target_box,
                #[weak]
                row,
                #[upgrade_or]
                None,
                move |_| {
                    preferences.move_up(&target_box, &dir_c, &row, k);
                    None
                }
            ),
        );

        let k = kind;
        let dir_c = dir;
        row.connect_local(
            "move-down",
            false,
            clone!(
                #[weak(rename_to=preferences)]
                self,
                #[weak]
                target_box,
                #[weak]
                row,
                #[upgrade_or]
                None,
                move |_| {
                    preferences.move_down(&target_box, &dir_c, &row, k);
                    None
                }
            ),
        );

        target_box.append(&row);
    }

    fn fix_movement_buttons(r: &mut [(String, dir_row::DirectoryRow)]) {
        let total = r.len();
        for (i, ro) in r.iter().enumerate() {
            if i == 0 {
                ro.1.set_up_enabled(false);
                ro.1.set_down_enabled(true);
            } else if i == total - 1 {
                ro.1.set_down_enabled(false);
                ro.1.set_up_enabled(true);
            } else {
                ro.1.set_up_enabled(true);
                ro.1.set_down_enabled(true);
            }
        }
    }

    fn remove(
        &self,
        target_box: &gtk4::Box,
        dir: &str,
        row: &dir_row::DirectoryRow,
        kind: DirectoryConfigType,
    ) {
        let self_ = self.imp();
        {
            let mut rows = self_.directory_rows.borrow_mut();
            target_box.remove(row);
            if let Some(r) = rows.get_mut(&kind) {
                r.retain(|i| i.0 != dir);
                Self::fix_movement_buttons(r);
            }
        }
        self.update_directories(kind);
    }

    fn move_up(
        &self,
        target_box: &gtk4::Box,
        dir: &str,
        row: &dir_row::DirectoryRow,
        kind: DirectoryConfigType,
    ) {
        let self_ = self.imp();
        {
            let mut rows = self_.directory_rows.borrow_mut();
            if let Some(r) = rows.get_mut(&kind) {
                let Some(current_position) = r.iter().position(|i| i.0 == dir) else {
                    return;
                };
                let item = r.remove(current_position);

                let sibling = &r[current_position - 1];
                target_box.reorder_child_after(&sibling.1, Some(&item.1));
                r.insert(current_position - 1, (dir.to_string(), row.clone()));

                Self::fix_movement_buttons(r);
            }
        }
        self.update_directories(kind);
    }

    fn move_down(
        &self,
        target_box: &gtk4::Box,
        dir: &str,
        row: &dir_row::DirectoryRow,
        kind: DirectoryConfigType,
    ) {
        let self_ = self.imp();
        {
            let mut rows = self_.directory_rows.borrow_mut();
            if let Some(r) = rows.get_mut(&kind) {
                let Some(current_position) = r.iter().position(|i| i.0 == dir) else {
                    return;
                };
                let item = r.remove(current_position);
                let total = r.len();
                if current_position < total {
                    let sibling = &r[current_position];
                    target_box.reorder_child_after(&item.1, Some(&sibling.1));
                    r.insert(current_position + 1, (dir.to_string(), row.clone()));
                }

                Self::fix_movement_buttons(r);
            }
        }
        self.update_directories(kind);
    }

    fn select_file(
        &self,
        filters: &'static [&str],
        title: &'static str,
    ) -> gtk4::FileChooserDialog {
        let self_ = self.imp();

        let native = gtk4::FileChooserDialog::new(
            Some(title),
            Some(self),
            gtk4::FileChooserAction::SelectFolder,
            &[
                ("Select", gtk4::ResponseType::Accept),
                ("Cancel", gtk4::ResponseType::Cancel),
            ],
        );

        native.set_modal(true);
        native.set_transient_for(Some(self));

        for f in filters {
            let filter = gtk4::FileFilter::new();
            filter.add_mime_type(f);
            filter.set_name(Some(f));
            native.add_filter(&filter);
        }

        self_.file_chooser.replace(Some(native.clone()));
        native.show();
        native
    }
}
