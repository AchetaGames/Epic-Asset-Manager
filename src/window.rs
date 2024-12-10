use crate::application::EpicAssetManager;
use crate::config::{APP_ID, PROFILE};
use crate::ui::update::Update;
use crate::ui::widgets::logged_in::refresh::Refresh;
use crate::ui::widgets::progress_icon::ProgressIconExt;
use crate::ui::PreferencesWindow;
use chrono::TimeZone;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*, ListBoxRow};
use gtk4::{gio, glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use log::{debug, error, warn};
use std::collections::HashMap;
use std::ops::Deref;

pub mod imp {
    use super::*;
    use crate::models::Model;
    use glib::ParamSpec;
    use gtk4::glib::ParamSpecString;
    use std::cell::RefCell;

    #[derive(CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/window.ui")]
    pub struct EpicAssetManagerWindow {
        #[template_child]
        pub headerbar: TemplateChild<adw::HeaderBar>,
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
        pub refresh: TemplateChild<gtk4::Button>,
        #[template_child]
        pub notifications: TemplateChild<gtk4::Box>,
        #[template_child]
        pub progress_button: TemplateChild<gtk4::MenuButton>,
        #[template_child]
        pub download_popover: TemplateChild<gtk4::Popover>,
        pub model: RefCell<Model>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicAssetManagerWindow {
        const NAME: &'static str = "EpicAssetManagerWindow";
        type Type = super::EpicAssetManagerWindow;
        type ParentType = gtk4::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

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
                refresh: TemplateChild::default(),
                notifications: TemplateChild::default(),
                progress_button: TemplateChild::default(),
                download_popover: TemplateChild::default(),
                model: RefCell::new(Model::new()),
            }
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EpicAssetManagerWindow {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("item").build(),
                    ParamSpecString::builder("product").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "item" => {
                    let item = value.get::<String>().unwrap();
                    self.logged_in_stack.set_property("item", item);
                }
                "product" => {
                    let product = value.get::<String>().unwrap();
                    self.logged_in_stack.set_property("product", product);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "item" => self.logged_in_stack.property("item"),
                "product" => self.logged_in_stack.property("product"),
                &_ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            // Devel Profile
            if PROFILE == "Devel" {
                obj.style_context().add_class("devel");
            }

            let button = self.color_scheme_btn.get();
            let style_manager = adw::StyleManager::default();

            if !style_manager.system_supports_color_schemes() {
                style_manager.connect_color_scheme_notify(move |style_manager| {
                    button.set_visible(true);
                    if style_manager.is_dark() {
                        button.set_icon_name("light-mode-symbolic");
                    } else {
                        button.set_icon_name("dark-mode-symbolic");
                    }
                });
            }

            // load latest window state
            obj.load_window_size();
            obj.setup_actions();
            obj.setup_receiver();
        }
    }

    impl WidgetImpl for EpicAssetManagerWindow {}

    impl WindowImpl for EpicAssetManagerWindow {
        // save window state on delete event
        fn close_request(&self) -> glib::Propagation {
            if let Err(err) = self.obj().save_window_size() {
                warn!("Failed to save window state, {}", &err);
            }

            // Pass close request on to the parent
            self.parent_close_request()
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
        let window: Self = glib::Object::new();
        window.set_application(Some(app));

        gtk4::Window::set_default_icon_name(APP_ID);

        window
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let self_ = self.imp();

        let settings = &self_.model.borrow().settings;

        let size = self.default_size();

        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;

        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let self_ = self.imp();

        let settings = &self_.model.borrow().settings;

        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let is_maximized = settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
        let style_manager = adw::StyleManager::default();
        let button = self_.color_scheme_btn.get();
        if style_manager.system_supports_color_schemes() {
            button.set_visible(false);
            if settings.boolean("dark-mode") {
                style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            } else {
                style_manager.set_color_scheme(adw::ColorScheme::Default);
            }
        } else {
            button.set_visible(true);
            if settings.boolean("dark-mode") {
                style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            } else {
                style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
            }
        }
    }

    pub fn setup_receiver(&self) {
        let self_ = self.imp();
        let receiver = self_
            .model
            .borrow()
            .deref()
            .receiver
            .borrow_mut()
            .take()
            .unwrap();
        glib::spawn_future_local(clone!(
            #[weak(rename_to=window)]
            self,
            #[upgrade_or_panic]
            async move {
                while let Ok(msg) = receiver.recv().await {
                    window.update(msg);
                }
            }
        ));
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        action!(
            self,
            "login",
            Some(&String::static_variant_type()),
            clone!(
                #[weak(rename_to=window)]
                self,
                move |_, sid_par| {
                    if let Some(sid_opt) = sid_par {
                        if let Some(sid) = sid_opt.get::<String>() {
                            window.login(sid);
                        }
                    }
                }
            )
        );

        self.insert_action_group("window", Some(self));

        action!(
            self,
            "logout",
            clone!(
                #[weak(rename_to=window)]
                self,
                move |_, _| {
                    window.logout();
                }
            )
        );

        action!(
            self,
            "refresh",
            clone!(
                #[weak(rename_to=window)]
                self,
                move |_, _| {
                    window.refresh();
                }
            )
        );

        self_.download_manager.connect_local(
            "tick",
            false,
            clone!(
                #[weak(rename_to=window)]
                self,
                #[upgrade_or]
                None,
                move |_| {
                    let self_ = window.imp();
                    self_
                        .progress_icon
                        .set_fraction(self_.download_manager.progress());
                    None
                }
            ),
        );
    }

    pub fn check_login(&mut self) {
        let self_ = self.imp();
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
        let self_ = self.imp();
        self_.sid_box.set_window(self);
        self_.refresh.set_visible(false);
        self_.logged_in_stack.activate(false);
        self_.main_stack.set_visible_child_name("sid_box");
        get_action!(self, @logout).set_enabled(false);
    }

    pub fn show_download_manager(&self) {
        let self_ = self.imp();
        self_.logged_in_stack.activate(false);
    }

    pub fn show_logged_in(&self) {
        let self_ = self.imp();
        get_action!(self, @logout).set_enabled(true);
        self_.logged_in_stack.activate(true);
        self_.refresh.set_visible(true);
        self_.main_stack.set_visible_child_name("logged_in_stack");
    }

    pub fn do_logout(&self) {
        let self_ = self.imp();
        self.save_secret(
            "bearer",
            "eam_epic_games_token",
            None,
            "token-expiration",
            None,
        );
        self.save_secret(
            "refresh",
            "eam_epic_games_refresh_token",
            None,
            "refresh-token-expiration",
            None,
        );
        self_.appmenu_button.set_label("");
        self_.appmenu_button.set_icon_name("open-menu-symbolic");
        self.show_login();
    }

    pub fn clear_notification(&self, name: &str) {
        let self_ = self.imp();
        if let Some(w) = self_.notifications.first_child() {
            if w.widget_name().eq(name) {
                self_.notifications.remove(&w);
            }
            while let Some(s) = w.next_sibling() {
                self_.notifications.remove(&s);
            }
        }
    }

    pub fn add_notification(&self, name: &str, message: &str, message_type: gtk4::MessageType) {
        let self_ = self.imp();
        self.clear_notification(name);
        let notif = gtk4::InfoBar::builder()
            .message_type(message_type)
            .name(name)
            .margin_start(10)
            .margin_end(10)
            .show_close_button(true)
            .build();
        let label = gtk4::Label::builder().label(message).build();
        notif.add_child(&label);
        notif.connect_response(clone!(
            #[weak]
            notif,
            #[weak(rename_to=window)]
            self,
            move |_, _| {
                let self_ = window.imp();
                self_.notifications.remove(&notif);
            }
        ));
        self_.notifications.append(&notif);
    }

    pub fn show_preferences(&self) -> PreferencesWindow {
        let preferences = PreferencesWindow::new();
        preferences.set_transient_for(Some(self));
        preferences.set_window(self);
        preferences.show();
        preferences
    }

    pub fn show_assets(&self, ud: &egs_api::api::types::account::UserData) {
        let self_ = self.imp();
        self_
            .model
            .borrow_mut()
            .epic_games
            .borrow_mut()
            .set_user_details(ud.clone());
        self_.refresh.set_visible(true);
        self_.logged_in_stack.set_window(self);
        self_.download_manager.set_window(self);
        self_
            .logged_in_stack
            .set_download_manager(&self_.download_manager);
        self.show_logged_in();
        let db = crate::models::database::connection();
        ud.display_name.as_ref().map_or_else(
            || {
                if let Ok(mut conn) = db.get() {
                    let data: Result<String, diesel::result::Error> =
                        crate::schema::user_data::table
                            .filter(crate::schema::user_data::name.eq("display_name"))
                            .select(crate::schema::user_data::value)
                            .first(&mut conn);
                    if let Ok(name) = data {
                        self_.appmenu_button.set_label(&name);
                    }
                }
            },
            |id| {
                if let Ok(mut conn) = db.get() {
                    diesel::replace_into(crate::schema::user_data::table)
                        .values((
                            crate::schema::user_data::name.eq("display_name"),
                            crate::schema::user_data::value.eq(id),
                        ))
                        .execute(&mut conn)
                        .expect("Unable to insert display name to the DB");
                };
                self_.appmenu_button.set_label(id);
            },
        );

        self.save_secret(
            ud.token_type
                .as_ref()
                .unwrap_or(&"login".to_string())
                .as_str(),
            "eam_epic_games_token",
            ud.access_token(),
            "token-expiration",
            ud.expires_at,
        );
        self.save_secret(
            "refresh",
            "eam_epic_games_refresh_token",
            ud.refresh_token(),
            "refresh-token-expiration",
            ud.refresh_expires_at,
        );
        self.show_logged_in();
        self_.logged_in_stack.set_window(self);
        self_.download_manager.set_window(self);
        self_
            .logged_in_stack
            .set_download_manager(&self_.download_manager);
    }

    pub fn create_info_row(text: &str) -> ListBoxRow {
        let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        b.set_margin_start(12);
        b.set_margin_end(12);
        b.set_margin_top(8);
        b.set_margin_bottom(8);
        let label = gtk4::Label::new(Some(&text));
        label.set_use_markup(true);
        label.set_selectable(true);
        label.set_wrap(true);
        b.append(&label);
        let row = gtk4::ListBoxRow::builder().activatable(false).child(&b);
        row.build()
    }

    pub fn create_widget_row(label: &str, widget: &impl IsA<gtk4::Widget>) -> ListBoxRow {
        let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        b.set_margin_start(12);
        b.set_margin_end(12);
        b.set_margin_top(8);
        b.set_margin_bottom(8);
        let label = gtk4::Label::new(Some(label));
        label.set_hexpand(true);
        label.set_halign(gtk4::Align::Start);
        label.set_valign(gtk4::Align::Center);
        label.set_selectable(true);
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        b.append(&label);
        widget.set_halign(gtk4::Align::End);
        b.append(widget);
        let row = gtk4::ListBoxRow::builder().activatable(false).child(&b);
        row.build()
    }

    fn save_secret(
        &self,
        secret_type: &str,
        secret_name: &str,
        secret: Option<String>,
        expiration_name: &str,
        expiration: Option<chrono::DateTime<chrono::Utc>>,
    ) {
        let self_ = self.imp();
        let mut attributes = HashMap::new();
        attributes.insert("application", crate::config::APP_ID);
        attributes.insert("type", secret_type);
        let d = expiration.map_or_else(
            || {
                chrono::Utc
                    .timestamp_opt(0, 0)
                    .unwrap()
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            },
            |e| e.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        );
        self_
            .model
            .borrow()
            .settings
            .set_string(expiration_name, d.as_str())
            .unwrap();

        #[cfg(target_os = "linux")]
        {
            debug!("Saving {} secret", secret_name);
            match &self_.model.borrow().secret_service {
                None => {
                    self.add_notification("ss_none_auth", "org.freedesktop.Secret.Service not available for use, secrets stored insecurely!", gtk4::MessageType::Warning);
                    self.save_insecure(secret_name, secret);
                }
                Some(ss) => {
                    // Clear the insecure storage if any
                    self_
                        .model
                        .borrow()
                        .settings
                        .set_string(
                            match secret_name {
                                "eam_epic_games_token" => "token",
                                "eam_epic_games_refresh_token" => "refresh-token",
                                _ => {
                                    return;
                                }
                            },
                            "",
                        )
                        .unwrap();
                    match secret {
                        None => {
                            if let Err(e) = ss.get_any_collection().unwrap().create_item(
                                secret_name,
                                attributes,
                                b"",
                                true,
                                "text/plain",
                            ) {
                                error!("Failed to save secret {}", e);
                                self.add_notification("ss_none_auth", "org.freedesktop.Secret.Service not available for use, secrets stored insecurely!", gtk4::MessageType::Warning);
                                self.save_insecure(secret_name, secret);
                            }
                        }
                        Some(rt) => {
                            if let Err(e) = ss.get_any_collection().unwrap().create_item(
                                secret_name,
                                attributes,
                                rt.as_bytes(),
                                true,
                                "text/plain",
                            ) {
                                error!("Failed to save secret {}", e);
                                self.add_notification("ss_none_auth", "org.freedesktop.Secret.Service not available for use, secrets stored insecurely!", gtk4::MessageType::Warning);
                                self.save_insecure(secret_name, Some(rt));
                            }
                        }
                    }
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            self.save_insecure(secret_name, secret);
        }
    }

    fn save_insecure(&self, secret_name: &str, secret: Option<String>) {
        let self_ = self.imp();
        self_
            .model
            .borrow()
            .settings
            .set_string(
                match secret_name {
                    "eam_epic_games_token" => "token",
                    "eam_epic_games_refresh_token" => "refresh-token",
                    _ => {
                        return;
                    }
                },
                &secret.unwrap_or_default(),
            )
            .unwrap();
    }

    pub fn close_download_manager(&self) {
        let self_ = self.imp();
        self_.progress_button.popdown();
        self_.download_popover.popdown();
    }

    pub fn refresh(&self) {
        let self_ = self.imp();
        self_.logged_in_stack.run_refresh();
    }
}
