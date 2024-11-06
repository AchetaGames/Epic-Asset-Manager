use glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use log::error;

pub mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::gio;
    use once_cell::sync::OnceCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/sid.ui")]
    pub struct SidBox {
        pub actions: gio::SimpleActionGroup,
        #[template_child]
        pub sid_entry: TemplateChild<gtk4::Entry>,
        pub window: OnceCell<EpicAssetManagerWindow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SidBox {
        const NAME: &'static str = "SidBox";
        type Type = super::SidBox;
        type ParentType = gtk4::Box;

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
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            obj.setup_events();
        }
    }

    impl WidgetImpl for SidBox {}
    impl BoxImpl for SidBox {}
}

glib::wrapper! {
    pub struct SidBox(ObjectSubclass<imp::SidBox>)
        @extends gtk4::Widget, gtk4::Box;
}

impl SidBox {
    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }
        self_.window.set(window.clone()).unwrap();
    }

    pub fn setup_events(&self) {
        let self_ = self.imp();
        self_.sid_entry.connect_changed(clone!(
            #[weak(rename_to=sid_box)]
            self,
            move |_| sid_box.validate_sid()
        ));
    }

    fn validate_sid(&self) {
        let self_ = self.imp();
        let text = self_.sid_entry.text();
        let is_valid = if text.len() == 32 {
            text.chars().all(char::is_alphanumeric)
        } else {
            false
        };
        get_action!(self_.actions, @login).set_enabled(is_valid);
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;

        self.insert_action_group("sid", Some(actions));

        action!(actions, "browser", move |_, _| {
            #[cfg(target_os = "linux")]
            if gio::AppInfo::launch_default_for_uri("https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect%3FclientId%3D34a02cf8f4414e29b15921876da36f9a%26responseType%3Dcode", None::<&gio::AppLaunchContext>).is_err() {
                error!("Please go to https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect%3FclientId%3D34a02cf8f4414e29b15921876da36f9a%26responseType%3Dcode");
            }
            #[cfg(target_os = "windows")]
            open::that("https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect%3FclientId%3D34a02cf8f4414e29b15921876da36f9a%26responseType%3Dcode");
        });

        action!(
            actions,
            "login",
            clone!(
                #[weak(rename_to=sid_box)]
                self,
                move |_, _| {
                    sid_box.login();
                }
            )
        );

        get_action!(self_.actions, @login).set_enabled(false);
    }

    fn login(&self) {
        let self_ = self.imp();
        let text = self_.sid_entry.text();
        if let Some(window) = self_.window.get() {
            gtk4::prelude::ActionGroupExt::activate_action(
                window,
                "login",
                Some(&text.to_variant()),
            );
        }
        self_.sid_entry.set_text("");
    }
}
