use gettextrs::gettext;
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
        pub unreal_engine_project_directories_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub unreal_engine_vault_directories_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub unreal_engine_directories_box: TemplateChild<gtk4::Box>,
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
                unreal_engine_project_directories_box: TemplateChild::default(),
                unreal_engine_vault_directories_box: TemplateChild::default(),
                unreal_engine_directories_box: TemplateChild::default(),
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
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
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
        let window: Self = glib::Object::new(&[]).expect("Failed to create PreferencesWindow");

        window
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);
        self_.window.set(window.clone()).unwrap();
    }

    pub fn imp(&self) -> &imp::PreferencesWindow {
        imp::PreferencesWindow::from_instance(self)
    }

    pub fn bind_settings(&self) {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);
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
    }

    // fn main_window(&self) -> Option<&crate::window::EpicAssetManagerWindow> {
    //     let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);
    //     match self_.window.get() {
    //         Some(window) => Some(&(*window)),
    //         None => None,
    //     }
    // }

    pub fn load_settings(&self) {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);
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
    }

    pub fn setup_actions(&self) {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);
        let actions = &self_.actions;

        self.insert_action_group("preferences", Some(actions));

        action!(
            actions,
            "cache",
            clone!(@weak self as win => move |_, _| {
                let dialog: gtk4::FileChooserDialog = win.select_file(&[], "Cache Directory");
                dialog.connect_response(clone!(@weak win => move |d, response| {
                    if response == gtk4::ResponseType::Accept {
                        if let Some(file) = d.file() {
                            win.set_directory(file, DirectoryConfigType::Cache);
                        }
                    }
                    d.destroy();
                }));
            })
        );

        action!(
            actions,
            "temp",
            clone!(@weak self as win => move |_, _| {
                let dialog: gtk4::FileChooserDialog = win.select_file(&[], "Temporary Directory");
                dialog.connect_response(clone!(@weak win => move |d, response| {
                    if response == gtk4::ResponseType::Accept {
                        if let Some(file) = d.file() {
                            win.set_directory(file, DirectoryConfigType::Temp);
                        }
                    }
                    d.destroy();
                }));
            })
        );
        action!(
            actions,
            "add_vault",
            clone!(@weak self as win => move |_, _| {
                let dialog: gtk4::FileChooserDialog = win.select_file(&[], "Vault Directory");
                dialog.connect_response(clone!(@weak win => move |d, response| {
                    if response == gtk4::ResponseType::Accept {
                        if let Some(file) = d.file() {
                            win.set_directory(file, DirectoryConfigType::Vault);
                        }
                    }
                    d.destroy();
                }));
            })
        );
        action!(
            actions,
            "add_engine",
            clone!(@weak self as win => move |_, _| {
                let dialog: gtk4::FileChooserDialog = win.select_file(&[], "Engine Directory");
                dialog.connect_response(clone!(@weak win => move |d, response| {
                    if response == gtk4::ResponseType::Accept {
                        if let Some(file) = d.file() {
                            win.set_directory(file, DirectoryConfigType::Engine);
                        }
                    }
                    d.destroy();
                }));
            })
        );
        action!(
            actions,
            "add_project",
            clone!(@weak self as win => move |_, _| {
                let dialog: gtk4::FileChooserDialog = win.select_file(&[], " Projects Directory");
                dialog.connect_response(clone!(@weak win => move |d, response| {
                    if response == gtk4::ResponseType::Accept {
                        if let Some(file) = d.file() {
                            win.set_directory(file, DirectoryConfigType::Projects);
                        }
                    }
                    d.destroy();
                }));
            })
        );
    }

    fn set_directory(&self, dir: File, kind: DirectoryConfigType) {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);
        match dir.query_file_type(FileQueryInfoFlags::NONE, gtk4::gio::NONE_CANCELLABLE) {
            FileType::Directory => {
                debug!("Selected Directory")
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
                    let n = match name.into_string() {
                        Ok(s) => s,
                        Err(_) => {
                            error!("Selected directory is not UTF8");
                            return;
                        }
                    };
                    if !current.contains(&gtk4::glib::GString::from(n.clone())) {
                        current.push(gtk4::glib::GString::from(n.clone()));
                        self.add_directory_row(widget, n, kind);
                    }
                    let new: Vec<&str> = current.iter().map(|i| i.as_str()).collect();
                    self_
                        .settings
                        .set_strv(setting_name, new.as_slice())
                        .unwrap()
                }
            }
            DirectoryConfigType::Games => {}
        };
    }

    fn update_directories(&self, kind: DirectoryConfigType) {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);
        let rows = self_.directory_rows.borrow();
        match rows.get(&kind) {
            None => {}
            Some(r) => {
                let v: Vec<&str> = r.iter().map(|i| i.0.as_str()).collect();
                if let Some(setting_name) = Self::setting_name_from_type(kind) {
                    self_.settings.set_strv(setting_name, v.as_slice()).unwrap()
                }
            }
        };
    }

    fn setting_name_from_type(kind: DirectoryConfigType) -> Option<&'static str> {
        match kind {
            DirectoryConfigType::Cache => None,
            DirectoryConfigType::Temp => None,
            DirectoryConfigType::Vault => Some("unreal-vault-directories"),
            DirectoryConfigType::Engine => Some("unreal-engine-directories"),
            DirectoryConfigType::Projects => Some("unreal-projects-directories"),
            DirectoryConfigType::Games => None,
        }
    }

    fn setting_name_and_box_from_type(
        &self,
        kind: DirectoryConfigType,
    ) -> Option<(&'static str, &gtk4::Box)> {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);
        match kind {
            DirectoryConfigType::Cache => None,
            DirectoryConfigType::Temp => None,
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
            DirectoryConfigType::Games => None,
        }
    }

    fn add_directory_row(&self, target_box: &gtk4::Box, dir: String, kind: DirectoryConfigType) {
        let row: super::dir_row::DirectoryRow =
            super::dir_row::DirectoryRow::new(dir.clone(), self);

        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);

        let mut rows = self_.directory_rows.borrow_mut();
        match rows.get_mut(&kind) {
            None => {
                row.set_up_enabled(false);
                row.set_down_enabled(false);
                rows.insert(kind, vec![(dir.clone(), row.clone())]);
            }
            Some(r) => {
                r.push((dir.clone(), row.clone()));
                let total = r.len();
                for (i, ro) in r.iter().enumerate() {
                    ro.1.set_up_enabled(true);
                    ro.1.set_down_enabled(true);
                    if i == 0 {
                        ro.1.set_up_enabled(false);
                    }
                    if i == total - 1 {
                        ro.1.set_down_enabled(false);
                    }
                }
            }
        };

        let k = kind;
        let dir_c = dir.clone();
        row.connect_local(
            "remove",
            false,
            clone!(@weak self as win, @weak target_box, @weak row => @default-return None, move |_| {
                let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(&win);
                {
                    let mut rows = self_.directory_rows.borrow_mut();
                    target_box.remove(&row);
                    if let Some(r) = rows.get_mut(&k) {
                        r.retain(|i| i.0 != dir_c);
                        let total = r.len();
                        for (i, ro) in r.iter().enumerate() {
                            ro.1.set_up_enabled(true);
                            ro.1.set_down_enabled(true);
                            if i == 0 {
                                ro.1.set_up_enabled(false);
                            }
                            if i == total-1 {
                                ro.1.set_down_enabled(false);
                            }
                        }
                    }
                }
                win.update_directories(kind);
                None
            }),
        )
        .unwrap();

        let k = kind;
        let dir_c = dir.clone();
        row.connect_local(
            "move-up",
            false,
            clone!(@weak self as win, @weak target_box, @weak row => @default-return None, move |_| {
                let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(&win);
                {
                    let mut rows = self_.directory_rows.borrow_mut();
                    if let Some(r) = rows.get_mut(&k) {
                        let current_position = match r.iter().position(|i| i.0 == dir_c) {
                            Some(p) => p,
                            None => return None
                        };
                        let item = r.remove(current_position);
                        let total = r.len();

                        let sibling = &r[current_position-1];
                        target_box.reorder_child_after(&sibling.1, Some(&item.1));
                        r.insert(current_position-1, (dir_c.clone(), row));

                        for (i, ro) in r.iter().enumerate() {
                            ro.1.set_up_enabled(true);
                            ro.1.set_down_enabled(true);
                            if i == 0 {
                                ro.1.set_up_enabled(false);
                            }
                            if i == total {
                                ro.1.set_down_enabled(false);
                            }
                        }

                    }
                }
                win.update_directories(kind);
                None
            }),
        ).unwrap();

        let k = kind;
        let dir_c = dir;
        row.connect_local(
            "move-down",
            false,
            clone!(@weak self as win, @weak target_box, @weak row => @default-return None, move |_| {
                let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(&win);
                {
                    let mut rows = self_.directory_rows.borrow_mut();
                    if let Some(r) = rows.get_mut(&k) {
                        let current_position = match r.iter().position(|i| i.0 == dir_c) {
                            Some(p) => p,
                            None => return None
                        };
                        let item = r.remove(current_position);
                        let total = r.len();
                        if current_position < total {
                            let sibling = &r[current_position];
                            target_box.reorder_child_after(&item.1, Some(&sibling.1));
                            r.insert(current_position+1, (dir_c.clone(), row));
                        }

                        for (i, ro) in r.iter().enumerate() {
                            ro.1.set_up_enabled(true);
                            ro.1.set_down_enabled(true);
                            if i == 0 {
                                ro.1.set_up_enabled(false);
                            }
                            if i == total {
                                ro.1.set_down_enabled(false);
                            }
                        }

                    }
                }
                win.update_directories(kind);
                None
            }),
        ).unwrap();

        target_box.append(&row);
    }

    fn select_file(
        &self,
        filters: &'static [&str],
        title: &'static str,
    ) -> gtk4::FileChooserDialog {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);

        let native = gtk4::FileChooserDialog::new(
            Some(&gettext(title)),
            Some(self),
            gtk4::FileChooserAction::SelectFolder,
            &[
                (&gettext("Select"), gtk4::ResponseType::Accept),
                (&gettext("Cancel"), gtk4::ResponseType::Cancel),
            ],
        );

        native.set_modal(true);
        native.set_transient_for(Some(self));

        filters.iter().for_each(|f| {
            let filter = gtk4::FileFilter::new();
            filter.add_mime_type(f);
            filter.set_name(Some(f));
            native.add_filter(&filter);
        });

        self_.file_chooser.replace(Some(native.clone()));
        native.show();
        native
    }
}
