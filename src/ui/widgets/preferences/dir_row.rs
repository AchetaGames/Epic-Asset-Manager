use glib::clone;
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::action;

pub mod imp {
    use super::*;
    use adw::subclass::action_row::ActionRowImpl;
    use glib::subclass::{self};
    use gtk::glib::subclass::Signal;
    use once_cell::sync::Lazy;
    use once_cell::sync::OnceCell;

    #[derive(CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/dir_row.ui")]
    pub struct DirectoryRow {
        pub window: OnceCell<crate::ui::widgets::preferences::window::PreferencesWindow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DirectoryRow {
        const NAME: &'static str = "DirectoryRow";
        type Type = super::DirectoryRow;
        type ParentType = adw::ActionRow;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
            }
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

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("remove", &[], <()>::static_type().into())
                    .flags(glib::SignalFlags::ACTION)
                    .build()]
            });
            SIGNALS.as_ref()
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
    pub fn new(
        dir: String,
        window: &crate::ui::widgets::preferences::window::PreferencesWindow,
    ) -> Self {
        let row: Self = glib::Object::new(&[]).expect("Failed to create DirectoryRow");
        adw::prelude::PreferencesRowExt::set_title(&row, Some(&dir));
        let self_: &imp::DirectoryRow = imp::DirectoryRow::from_instance(&row);
        self_.window.set(window.clone()).unwrap();
        let actions = gio::SimpleActionGroup::new();

        row.insert_action_group("dir_row", Some(&actions));

        action!(
            actions,
            "remove",
            clone!(@weak row as row => move |_, _| {
                row.emit_by_name("remove", &[]).unwrap();
            })
        );
        row
    }
}
