use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{self, gio, prelude::*};
use gtk::{glib, CompositeTemplate};
use gtk_macros::action;
use log::error;

pub(crate) mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk::gio;
    use once_cell::sync::OnceCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/sid.ui")]
    pub struct SidBox {
        pub actions: gio::SimpleActionGroup,
        #[template_child]
        pub sid_entry: TemplateChild<gtk::Text>,
        pub window: OnceCell<EpicAssetManagerWindow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SidBox {
        const NAME: &'static str = "SidBox";
        type Type = super::SidBox;
        type ParentType = gtk::Box;

        fn new() -> Self {
            Self {
                actions: gio::SimpleActionGroup::new(),
                sid_entry: TemplateChild::default(),
                window: OnceCell::new(),
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

    impl ObjectImpl for SidBox {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
        }
    }

    impl WidgetImpl for SidBox {}
    impl BoxImpl for SidBox {}
}

glib::wrapper! {
    pub struct SidBox(ObjectSubclass<imp::SidBox>)
        @extends gtk::Widget, gtk::Box;
}

impl SidBox {
    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = imp::SidBox::from_instance(self);
        self_.window.set(window.clone()).unwrap();
    }

    pub fn setup_actions(&self) {
        let self_ = imp::SidBox::from_instance(self);
        let actions = &self_.actions;

        self.insert_action_group("sid", Some(actions));
        action!(actions, "browser", move |_, _| {
            if let Err(_) = gio::AppInfo::launch_default_for_uri("https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect", None::<&gio::AppLaunchContext>) {
            error!("Please go to https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect")
        }
        });

        action!(
            actions,
            "login",
            clone!(@weak self as sid_box => move |_, _| {
                let self_: &crate::ui::widgets::sid_login::imp::SidBox = imp::SidBox::from_instance(&sid_box);
                let text = self_.sid_entry.text();

                if let Some(window) = self_.window.get() {
                        gtk::prelude::ActionGroupExt::activate_action(window, "login", Some(&text.to_variant()));
                }
                self_.sid_entry.set_text("");
            })
        );
    }
}
