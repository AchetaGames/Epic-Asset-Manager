use glib::clone;
use gtk4::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};

pub mod imp {
    use super::*;
    use adw::subclass::action_row::ActionRowImpl;
    use adw::subclass::prelude::PreferencesRowImpl;
    use glib::subclass::{self};
    use gtk4::glib::subclass::Signal;
    use once_cell::sync::Lazy;
    use once_cell::sync::OnceCell;

    #[derive(CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/dir_row.ui")]
    pub struct DirectoryRow {
        pub window: OnceCell<crate::ui::widgets::preferences::PreferencesWindow>,
        pub actions: gio::SimpleActionGroup,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DirectoryRow {
        const NAME: &'static str = "DirectoryRow";
        type Type = super::DirectoryRow;
        type ParentType = adw::ActionRow;

        fn new() -> Self {
            Self {
                actions: gio::SimpleActionGroup::new(),
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
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("remove")
                        .flags(glib::SignalFlags::ACTION)
                        .build(),
                    Signal::builder("move-up")
                        .flags(glib::SignalFlags::ACTION)
                        .build(),
                    Signal::builder("move-down")
                        .flags(glib::SignalFlags::ACTION)
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for DirectoryRow {}
    impl ActionRowImpl for DirectoryRow {}
    impl PreferencesRowImpl for DirectoryRow {}
    impl ListBoxRowImpl for DirectoryRow {}
}

glib::wrapper! {
    pub struct DirectoryRow(ObjectSubclass<imp::DirectoryRow>)
        @extends gtk4::Widget, gtk4::ListBoxRow, adw::ActionRow, adw::PreferencesRow;
}

impl DirectoryRow {
    pub fn new(dir: &str, window: &crate::ui::widgets::preferences::PreferencesWindow) -> Self {
        let row: Self = glib::Object::new();
        adw::prelude::PreferencesRowExt::set_title(&row, dir);
        let self_ = row.imp();
        self_.window.set(window.clone()).unwrap();

        row.insert_action_group("dir_row", Some(&self_.actions));

        action!(
            self_.actions,
            "remove",
            clone!(
                #[weak]
                row,
                move |_, _| {
                    row.emit_by_name::<()>("remove", &[]);
                }
            )
        );

        action!(
            self_.actions,
            "up",
            clone!(
                #[weak]
                row,
                move |_, _| {
                    row.emit_by_name::<()>("move-up", &[]);
                }
            )
        );

        action!(
            self_.actions,
            "down",
            clone!(
                #[weak]
                row,
                move |_, _| {
                    row.emit_by_name::<()>("move-down", &[]);
                }
            )
        );
        row.set_down_enabled(false);
        row
    }

    pub fn set_up_enabled(&self, enabled: bool) {
        let self_ = self.imp();
        get_action!(self_.actions, @up).set_enabled(enabled);
    }

    pub fn set_down_enabled(&self, enabled: bool) {
        let self_ = self.imp();
        get_action!(self_.actions, @down).set_enabled(enabled);
    }
}
