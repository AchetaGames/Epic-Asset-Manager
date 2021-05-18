use adw::ActionRow;
use gtk::gio::SettingsBindFlags;
use gtk::pango::EllipsizeMode;
use gtk::{gio, glib, prelude::*, subclass::prelude::*, Align, CompositeTemplate, Orientation};
use once_cell::sync::OnceCell;

pub mod imp {
    use super::*;
    use adw::subclass::action_row::ActionRowImpl;
    use adw::subclass::preferences_row::PreferencesRowImpl;
    use glib::subclass::{self};

    #[derive(CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/dir_row.ui")]
    pub struct DirectoryRow {}

    #[glib::object_subclass]
    impl ObjectSubclass for DirectoryRow {
        const NAME: &'static str = "DirectoryRow";
        type Type = super::DirectoryRow;
        type ParentType = adw::ActionRow;

        fn new() -> Self {
            Self {}
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DirectoryRow {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for DirectoryRow {}
    impl ActionRowImpl for DirectoryRow {}
    impl ListBoxRowImpl for DirectoryRow {}
}

glib::wrapper! {
    pub struct DirectoryRow(ObjectSubclass<imp::DirectoryRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::ActionRow, adw::PreferencesRow;
}

impl DirectoryRow {
    pub fn new(dir: String) -> Self {
        let row: Self = glib::Object::new(&[]).expect("Failed to create DirectoryRow");
        adw::prelude::PreferencesRowExt::set_title(&row, Some(&dir));
        row
    }
}
