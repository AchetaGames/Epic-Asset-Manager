use crate::application::EpicAssetManager;
use crate::config::{APP_ID, PROFILE};
use crate::ui::update::Update;
use crate::ui::widgets::progress_icon::ProgressIconExt;
use glib::clone;
use glib::signal::Inhibit;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{gio, glib, CompositeTemplate};
use gtk_macros::action;
use log::{debug, error, warn};
use std::collections::HashMap;
use std::ops::Deref;

pub(crate) mod imp {
    use super::*;
    use crate::models::Model;
    use glib::ParamSpec;
    use std::cell::RefCell;

    #[derive(CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/window.ui")]
    pub struct EpicAssetManagerWindow {
        #[template_child]
        pub headerbar: TemplateChild<gtk4::HeaderBar>,
        #[template_child]
        pub main_stack: TemplateChild<gtk4::Stack>,
        #[template_child]
        pub logged_in_stack: TemplateChild<crate::ui::widgets::logged_in::EpicLoggedInBox>,
        #[template_child]
        pub sid_box: TemplateChild<crate::ui::widgets::sid_login::SidBox>,
        #[template_child]
        pub progress_message: TemplateChild<gtk4::Label>,
        #[template_child]
        pub download_manager:
            TemplateChild<crate::ui::widgets::download_manager::EpicDownloadManager>,
        #[template_child]
        pub progress_icon: TemplateChild<crate::ui::widgets::progress_icon::ProgressIcon>,
        #[template_child]
        pub appmenu_button: TemplateChild<gtk4::MenuButton>,
        #[template_child]
        pub color_scheme_btn: TemplateChild<gtk4::Button>,
        #[template_child]
        pub notifications: TemplateChild<gtk4::Box>,
        pub model: RefCell<Model>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicAssetManagerWindow {
        const NAME: &'static str = "EpicAssetManagerWindow";
        type Type = super::EpicAssetManagerWindow;
        type ParentType = gtk4::ApplicationWindow;

        fn new() -> Self {
            Self {
                headerbar: TemplateChild::default(),
                main_stack: TemplateChild::default(),
                logged_in_stack: TemplateChild::default(),
                sid_box: TemplateChild::default(),
                progress_message: TemplateChild::default(),
                download_manager: TemplateChild::default(),
                progress_icon: TemplateChild::default(),
                appmenu_button: TemplateChild::default(),
                color_scheme_btn: TemplateChild::default(),
                notifications: TemplateChild::default(),
                model: RefCell::new(Model::new()),
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

    impl ObjectImpl for EpicAssetManagerWindow {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            // Devel Profile
            if PROFILE == "Devel" {
                obj.style_context().add_class("devel");
            }

            let button = self.color_scheme_btn.get();
            let style_manager = adw::StyleManager::default().unwrap();

            style_manager.connect_color_scheme_notify(move |style_manager| {
                let supported = style_manager.system_supports_color_schemes();
                button.set_visible(!supported);
                if supported {
                    style_manager.set_color_scheme(adw::ColorScheme::Default);
                } else {
                    if style_manager.is_dark() {
                        button.set_icon_name("light-mode-symbolic");
                    } else {
                        button.set_icon_name("dark-mode-symbolic");
                    }
                }
            });

            // load latest window state
            obj.load_window_size();
            obj.setup_actions();
            obj.setup_receiver();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_string(
                        "item",
                        "item",
                        "item",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "product",
                        "product",
                        "product",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "item" => {
                    let item = value.get::<String>().unwrap();
                    self.logged_in_stack.set_property("item", item).unwrap();
                }
                "product" => {
                    let product = value.get::<String>().unwrap();
                    self.logged_in_stack
                        .set_property("product", product)
                        .unwrap();
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "item" => self
                    .logged_in_stack
                    .property("item")
                    .unwrap_or_else(|_| "".to_value())
                    .to_value(),
                "product" => self
                    .logged_in_stack
                    .property("product")
                    .unwrap_or_else(|_| "".to_value())
                    .to_value(),
                &_ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicAssetManagerWindow {}

    impl WindowImpl for EpicAssetManagerWindow {
        // save window state on delete event
        fn close_request(&self, obj: &Self::Type) -> Inhibit {
            if let Err(err) = obj.save_window_size() {
                warn!("Failed to save window state, {}", &err);
            }
            Inhibit(false)
        }
    }

    impl ApplicationWindowImpl for EpicAssetManagerWindow {}
}

glib::wrapper! {
    pub struct EpicAssetManagerWindow(ObjectSubclass<imp::EpicAssetManagerWindow>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow, gio::ActionMap, gio::ActionGroup;
}

impl EpicAssetManagerWindow {
    pub fn new(app: &EpicAssetManager) -> Self {
        let window: Self = glib::Object::new(&[]).expect("Failed to create EpicAssetManagerWindow");
        window.set_application(Some(app));

        gtk4::Window::set_default_icon_name(APP_ID);

        window
    }

    pub fn data(&self) -> &imp::EpicAssetManagerWindow {
        imp::EpicAssetManagerWindow::from_instance(self)
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();

        let settings = &self_.model.borrow().settings;

        let size = self.default_size();

        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;

        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();

        let settings = &self_.model.borrow().settings;

        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let is_maximized = settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
        let style_manager = adw::StyleManager::default().unwrap();
        if !style_manager.system_supports_color_schemes() {
            if settings.boolean("dark-mode") {
                style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            } else {
                style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
            }
        }
    }

    pub fn setup_receiver(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        self_
            .model
            .borrow()
            .deref()
            .receiver
            .borrow_mut()
            .take()
            .unwrap()
            .attach(
                None,
                clone!(@weak self as window => @default-panic, move |msg| {
                    window.update(msg);
                    glib::Continue(true)
                }),
            );
    }

    pub fn setup_actions(&self) {
        action!(
            self,
            "login",
            Some(&String::static_variant_type()),
            clone!(@weak self as window => move |_, sid_par| {
                if let Some(sid_opt) = sid_par {
                    if let Some(sid) = sid_opt.get::<String>() {
                        window.login(sid);
                    }
                }
            })
        );
        let self_: &imp::EpicAssetManagerWindow = imp::EpicAssetManagerWindow::from_instance(self);

        self_.download_manager.connect_local(
            "tick",
            false,
            clone!(@weak self as obj => @default-return None, move |_| {
                let self_: &imp::EpicAssetManagerWindow = imp::EpicAssetManagerWindow::from_instance(&obj);
                self_.progress_icon.set_fraction(self_.download_manager.progress());
                None}),
        )
            .unwrap();
    }

    pub fn check_login(&mut self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        self_.main_stack.set_visible_child_name("progress");
        self_.progress_message.set_text("Loading");
        if self.can_relogin() {
            self_.progress_message.set_text("Resuming session");
            self.relogin();
        } else {
            self.show_login();
        }
    }

    pub fn show_login(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        self_.sid_box.set_window(self);
        self_.logged_in_stack.activate(false);
        self_.main_stack.set_visible_child_name("sid_box");
    }

    pub fn show_download_manager(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        self_.logged_in_stack.activate(false);
        // self_.main_stack.set_visible_child_name("download_manager")
    }

    pub fn show_logged_in(&self) {
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();
        self_.logged_in_stack.activate(true);
        self_.main_stack.set_visible_child_name("logged_in_stack");
    }

    pub fn add_notification(&self, message: &str, message_type: gtk4::MessageType) {
        let self_: &crate::window::imp::EpicAssetManagerWindow =
            crate::window::imp::EpicAssetManagerWindow::from_instance(self);
        let notif = gtk4::InfoBarBuilder::new()
            .message_type(message_type)
            .margin_start(10)
            .margin_end(10)
            .show_close_button(true)
            .build();
        let label = gtk4::LabelBuilder::new().label(message).build();
        notif.add_child(&label);
        notif.connect_response(
            clone!(@weak notif, @weak self as window => @default-panic, move |_, _| {
                let self_: &crate::window::imp::EpicAssetManagerWindow =
            crate::window::imp::EpicAssetManagerWindow::from_instance(&window);
                self_.notifications.remove(&notif);
            }),
        );
        self_.notifications.append(&notif);
    }

    pub fn show_assets(&self, ud: &egs_api::api::UserData) {
        // TODO display user information from the UserData
        let self_: &crate::window::imp::EpicAssetManagerWindow =
            crate::window::imp::EpicAssetManagerWindow::from_instance(self);
        self_
            .model
            .borrow_mut()
            .epic_games
            .borrow_mut()
            .set_user_details(ud.clone());
        self_.logged_in_stack.set_window(self);
        self_.download_manager.set_window(self);
        self_
            .logged_in_stack
            .set_download_manager(&self_.download_manager);
        self.show_logged_in();
        if let Some(id) = &ud.display_name {
            self_.appmenu_button.set_label(id);
        }
        if let Some(t) = ud.token_type.clone() {
            let mut attributes = HashMap::new();
            attributes.insert("application", crate::config::APP_ID);
            attributes.insert("type", t.as_str());
            if let Some(e) = ud.expires_at {
                let d = e.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                self_
                    .model
                    .borrow()
                    .settings
                    .set_string("token-expiration", d.as_str())
                    .unwrap();
                if let Some(at) = ud.access_token() {
                    debug!("Saving token secret");
                    if let Err(e) = self_
                        .model
                        .borrow()
                        .secret_service
                        .get_any_collection()
                        .unwrap()
                        .create_item(
                            "eam_epic_games_token",
                            attributes.clone(),
                            at.as_bytes(),
                            true,
                            "text/plain",
                        )
                    {
                        error!("Failed to save secret {}", e);
                    };
                }
            }
            let mut attributes = HashMap::new();
            attributes.insert("application", crate::config::APP_ID);
            attributes.insert("type", "refresh");
            if let Some(e) = ud.refresh_expires_at {
                let d = e.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                self_
                    .model
                    .borrow()
                    .settings
                    .set_string("refresh-token-expiration", d.as_str())
                    .unwrap();
                if let Some(rt) = ud.refresh_token() {
                    debug!("Saving refresh token secret");
                    if let Err(e) = self_
                        .model
                        .borrow()
                        .secret_service
                        .get_any_collection()
                        .unwrap()
                        .create_item(
                            "eam_epic_games_refresh_token",
                            attributes,
                            rt.as_bytes(),
                            true,
                            "text/plain",
                        )
                    {
                        error!("Failed to save secret {}", e);
                    };
                }
            }
        }
        self.show_logged_in();
        self_.logged_in_stack.set_window(self);
        self_.download_manager.set_window(self);
        self_
            .logged_in_stack
            .set_download_manager(&self_.download_manager);
    }
}
