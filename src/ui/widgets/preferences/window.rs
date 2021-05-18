use gettextrs::gettext;
use glib::clone;
use gtk::gdk_pixbuf::gio::{FileType, Settings};
use gtk::gio::{File, FileQueryInfoFlags, SettingsBindFlags};
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};
use log::{debug, error};
use once_cell::sync::OnceCell;
use std::ffi::OsString;

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
        pub file_chooser: RefCell<Option<gtk::FileChooserDialog>>,
        #[template_child]
        pub cache_directory_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub temp_directory_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub unreal_engine_project_directories_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub unreal_engine_vault_directories_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub unreal_engine_directories_box: TemplateChild<gtk::Box>,
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
        @extends gtk::Widget, gtk::Window, adw::Window, adw::PreferencesWindow;
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum DirectoryConfigType {
    Cache,
    Temp,
    Vault,
    Engine,
    Projects,
    Games,
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

    fn main_window(&self) -> Option<&crate::window::EpicAssetManagerWindow> {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);
        match self_.window.get() {
            Some(window) => Some(&(*window)),
            None => None,
        }
    }

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
                let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(&win);
                let dialog: gtk::FileChooserDialog = win.select_file(&[], "Cache Directory");
                dialog.connect_response(clone!(@weak win => move |d, response| {
                    if response == gtk::ResponseType::Accept {
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
                let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(&win);
                let dialog: gtk::FileChooserDialog = win.select_file(&[], "Temporary Directory");
                dialog.connect_response(clone!(@weak win => move |d, response| {
                    if response == gtk::ResponseType::Accept {
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
                let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(&win);
                let dialog: gtk::FileChooserDialog = win.select_file(&[], "Vault Directory");
                dialog.connect_response(clone!(@weak win => move |d, response| {
                    if response == gtk::ResponseType::Accept {
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
                let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(&win);
                let dialog: gtk::FileChooserDialog = win.select_file(&[], "Vault Directory");
                dialog.connect_response(clone!(@weak win => move |d, response| {
                    if response == gtk::ResponseType::Accept {
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
                let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(&win);
                let dialog: gtk::FileChooserDialog = win.select_file(&[], "Vault Directory");
                dialog.connect_response(clone!(@weak win => move |d, response| {
                    if response == gtk::ResponseType::Accept {
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
        match dir.query_file_type(FileQueryInfoFlags::NONE, gtk::gio::NONE_CANCELLABLE) {
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
            DirectoryConfigType::Vault => {
                let mut current = self_.settings.strv("unreal-vault-directories");
                let n = match name.into_string() {
                    Ok(s) => s,
                    Err(_) => {
                        error!("Selected directory is not UTF8");
                        return;
                    }
                };
                if !current.contains(&gtk::glib::GString::from(n.clone())) {
                    current.push(gtk::glib::GString::from(n.clone()));
                    self.add_directory_row(
                        &self_.unreal_engine_vault_directories_box,
                        n,
                        DirectoryConfigType::Vault,
                    );
                }
                let new: Vec<&str> = current.iter().map(|i| i.as_str()).collect();
                self_
                    .settings
                    .set_strv("unreal-vault-directories", &new.as_slice())
                    .unwrap()
            }
            DirectoryConfigType::Engine => {}
            DirectoryConfigType::Projects => {}
            DirectoryConfigType::Games => {}
        };
    }

    fn unset_directory(&self, dir: String, kind: DirectoryConfigType) {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);
        match kind {
            DirectoryConfigType::Cache => {}
            DirectoryConfigType::Temp => {}
            DirectoryConfigType::Vault => {
                let mut current = self_.settings.strv("unreal-vault-directories");
                current.retain(|s| !s.eq(&dir));
                let new: Vec<&str> = current.iter().map(|i| i.as_str()).collect();
                self_
                    .settings
                    .set_strv("unreal-vault-directories", &new.as_slice())
                    .unwrap()
            }
            DirectoryConfigType::Engine => {}
            DirectoryConfigType::Projects => {}
            DirectoryConfigType::Games => {}
        }
    }

    fn add_directory_row(&self, target_box: &gtk::Box, dir: String, kind: DirectoryConfigType) {
        let row: super::dir_row::DirectoryRow =
            super::dir_row::DirectoryRow::new(dir.clone(), &self);

        row.connect_local(
            "remove",
            false,
            clone!(@weak self as win, @weak target_box, @weak row => @default-return None, move |_| {
                target_box.remove(&row);
                win.unset_directory(dir.clone(), kind.clone());
                None
            }),
        )
        .unwrap();
        target_box.append(&row);
    }

    fn select_file(&self, filters: &'static [&str], title: &'static str) -> gtk::FileChooserDialog {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);

        let native = gtk::FileChooserDialog::new(
            Some(&gettext(title)),
            Some(self),
            gtk::FileChooserAction::SelectFolder,
            &[
                (&gettext("Select"), gtk::ResponseType::Accept),
                (&gettext("Cancel"), gtk::ResponseType::Cancel),
            ],
        );

        native.set_modal(true);
        native.set_transient_for(Some(self));

        filters.iter().for_each(|f| {
            let filter = gtk::FileFilter::new();
            filter.add_mime_type(f);
            filter.set_name(Some(f));
            native.add_filter(&filter);
        });

        self_.file_chooser.replace(Some(native.clone()));
        native.show();
        native
    }
}
