use gtk::gio::SettingsBindFlags;
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate, Orientation};
use once_cell::sync::OnceCell;

pub mod imp {
    use super::*;
    use adw::subclass::{preferences_window::PreferencesWindowImpl, window::AdwWindowImpl};
    use glib::subclass::{self};

    #[derive(CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/preferences.ui")]
    pub struct PreferencesWindow {
        pub settings: gio::Settings,
        #[template_child]
        pub cache_directory_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub temp_directory_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub unreal_engine_project_directories_box: TemplateChild<gtk::Box>,
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
                cache_directory_row: TemplateChild::default(),
                temp_directory_row: TemplateChild::default(),
                unreal_engine_project_directories_box: TemplateChild::default(),
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

impl PreferencesWindow {
    pub fn new() -> Self {
        let window: Self = glib::Object::new(&[]).expect("Failed to create PreferencesWindow");

        window
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

    pub fn load_settings(&self) {
        let self_: &imp::PreferencesWindow = imp::PreferencesWindow::from_instance(self);
        for dir in self_.settings.strv("unreal-projects-directories") {
            println!("Adding {}", dir);
            self.add_directory_row(
                &self_.unreal_engine_project_directories_box,
                dir.to_string(),
            );
        }
    }

    fn add_directory_row(&self, target_box: &gtk::Box, dir: String) {
        let container = gtk::BoxBuilder::new()
            .orientation(Orientation::Horizontal)
            .build();
        container.append(&gtk::Label::new(Some(&dir)));
        let remove = gtk::Button::from_icon_name(Some("list-remove"));
        container.append(&remove);
        let up = gtk::Button::from_icon_name(Some("go-up-symbolic"));
        container.append(&up);
        let down = gtk::Button::from_icon_name(Some("go-down-symbolic"));
        container.append(&down);
        target_box.append(&container);
    }
}
