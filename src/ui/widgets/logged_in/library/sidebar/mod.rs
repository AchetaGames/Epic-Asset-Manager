use crate::ui::widgets::logged_in::library::sidebar::categories::EpicSidebarCategories;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;
use log::{error, warn};
use std::thread;
use tokio::runtime::Builder;

pub mod button;
pub mod categories;
mod category;

pub mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/sidebar.ui")]
    pub struct EpicSidebar {
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub settings: gtk4::gio::Settings,
        pub loggedin: OnceCell<crate::ui::widgets::logged_in::library::EpicLibraryBox>,
        pub expanded: RefCell<bool>,
        #[template_child]
        pub expand_button: TemplateChild<gtk4::ToggleButton>, // TODO: Somehow use the EpicAssetManagerWindow's expand_button instead
        #[template_child]
        pub expand_image: TemplateChild<gtk4::Image>,
        #[template_child]
        pub stack: TemplateChild<gtk4::Stack>,
        #[template_child]
        pub all_category: TemplateChild<button::EpicSidebarButton>,
        #[template_child]
        pub unreal_category: TemplateChild<button::EpicSidebarButton>,
        #[template_child]
        pub games_category: TemplateChild<button::EpicSidebarButton>,
        #[template_child]
        pub downloaded_filter: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub favorites_filter: TemplateChild<gtk4::ToggleButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicSidebar {
        const NAME: &'static str = "EpicSidebar";
        type Type = super::EpicSidebar;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                actions: gio::SimpleActionGroup::new(),
                download_manager: OnceCell::new(),
                window: OnceCell::new(),
                loggedin: OnceCell::new(),
                expanded: RefCell::new(false),
                expand_button: TemplateChild::default(),
                expand_image: TemplateChild::default(),
                stack: TemplateChild::default(),
                all_category: TemplateChild::default(),
                unreal_category: TemplateChild::default(),
                games_category: TemplateChild::default(),
                downloaded_filter: TemplateChild::default(),
                favorites_filter: TemplateChild::default(),
                settings: gio::Settings::new(crate::config::APP_ID),
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

    impl ObjectImpl for EpicSidebar {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            self.all_category.set_sidebar(&obj);
            self.unreal_category.set_sidebar(&obj);
            self.games_category.set_sidebar(&obj);
            obj.setup_widgets();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;

            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![ParamSpecBoolean::builder("expanded").build()]);
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "expanded" => {
                    let sidebar_expanded = value.get().unwrap();
                    self.expanded.replace(sidebar_expanded);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "expanded" => self.expanded.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicSidebar {}
    impl BoxImpl for EpicSidebar {}
}

glib::wrapper! {
    pub struct EpicSidebar(ObjectSubclass<imp::EpicSidebar>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicSidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicSidebar {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    pub fn set_logged_in(&self, loggedin: &crate::ui::widgets::logged_in::library::EpicLibraryBox) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.loggedin.get().is_some() {
            return;
        }

        self_.loggedin.set(loggedin.clone()).unwrap();
        match self_.settings.string("default-category").as_str() {
            "all" => &self_.all_category,
            "games" => &self_.games_category,
            _ => &self_.unreal_category,
        }
        .clicked();
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("sidebar", Some(actions));

        action!(
            self_.actions,
            "expand",
            clone!(
                #[weak(rename_to=sidebar)]
                self,
                move |_, _| {
                    sidebar.expand();
                }
            )
        );
        action!(
            self_.actions,
            "marketplace",
            clone!(
                #[weak(rename_to=sidebar)]
                self,
                move |_, _| {
                    sidebar.open_marketplace();
                }
            )
        );
    }

    fn open_marketplace(&self) {
        let self_ = self.imp();
        if let Some(window) = self_.window.get() {
            let win_ = window.imp();
            let mut eg = win_.model.borrow().epic_games.borrow().clone();
            let (sender, receiver) = async_channel::unbounded::<String>();

            glib::spawn_future_local(async move {
                while let Ok(response) = receiver.recv().await {
                    open_browser(&response);
                }
            });

            thread::spawn(move || {
                match Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(eg.game_token())
                {
                    None => {}
                    Some(token) => {
                        sender.send_blocking(token.code).unwrap();
                    }
                }
            });
        }
    }

    pub fn setup_widgets(&self) {
        let self_ = self.imp();

        // Unreal Category
        let c = categories::EpicSidebarCategories::new(
            "Unreal Engine",
            "unreal",
            Some("assets|projects|plugins|engines"),
            Some(self),
        );
        c.set_widget_name("unreal");
        self_.stack.add_named(&c, Some("unreal"));
        // Games Category
        let c =
            categories::EpicSidebarCategories::new("Games", "games", Some("games|dlc"), Some(self));
        c.set_widget_name("games");
        self_.stack.add_named(&c, Some("games"));
        // All Category
        let c = categories::EpicSidebarCategories::new("All", "all", None, Some(self));
        c.set_widget_name("all");
        self_.stack.add_named(&c, Some("all"));

        self_.favorites_filter.connect_toggled(clone!(
            #[weak(rename_to=sidebar)]
            self,
            move |_| {
                sidebar.filter_changed();
            }
        ));
        self_.downloaded_filter.connect_toggled(clone!(
            #[weak(rename_to=sidebar)]
            self,
            move |_| {
                sidebar.filter_changed();
            }
        ));

        if self_.settings.boolean("sidebar-expanded") {
            self.expand();
        };
    }

    fn category_by_name(&self, name: &str) -> Option<EpicSidebarCategories> {
        let self_ = self.imp();
        if let Some(w) = self_.stack.child_by_name(name) {
            return w
                .downcast_ref::<categories::EpicSidebarCategories>()
                .cloned();
        }
        None
    }

    pub fn expanded(&self) -> bool {
        self.property("expanded")
    }

    pub fn expand(&self) {
        let self_ = self.imp();
        let new_value = !self.expanded();
        if new_value {
            self_
                .expand_image
                .set_icon_name(Some("go-previous-symbolic"));
            self_
                .expand_button
                .set_tooltip_text(Some("Collapse Sidebar"));
        } else {
            self_.expand_image.set_icon_name(Some("go-next-symbolic"));
            self_.expand_button.set_tooltip_text(Some("Expand Sidebar"));
        };
        if let Err(e) = self_.settings.set_boolean("sidebar-expanded", new_value) {
            warn!("Unable to save sidebar state: {}", e);
        };
        self.set_property("expanded", new_value);
        self_.all_category.set_property("expanded", new_value);
        self_.unreal_category.set_property("expanded", new_value);
        self_.games_category.set_property("expanded", new_value);
    }

    pub fn filter_changed(&self) {
        let self_ = self.imp();
        if let Some(s) = self_.stack.visible_child_name() {
            self.set_filter(None, Some(s.to_string()));
        }
    }

    pub fn set_filter(&self, filter: Option<String>, path: Option<String>) {
        let self_ = self.imp();
        if let Some(p) = path {
            if self_.stack.child_by_name(&p).is_some() {
                self_.stack.set_visible_child_name(&p);
            }
            if let Some(l) = self_.loggedin.get() {
                self.category_by_name(&p).map_or_else(
                    || {
                        l.set_property("filter", filter);
                    },
                    |cat| {
                        let filter = cat.filter().map(|filter| {
                            let mut prefix = String::new();
                            if self_.downloaded_filter.is_active() {
                                prefix.push_str("downloaded&");
                            }
                            if self_.favorites_filter.is_active() {
                                prefix.push_str("favorites&");
                            }
                            format!("{prefix}{filter}")
                        });
                        l.set_property("filter", filter);
                    },
                );
            };
        }
    }

    pub fn activate_all_buttons(&self) {
        let self_ = self.imp();
        self_.all_category.activate(true);
        self_.unreal_category.activate(true);
        self_.games_category.activate(true);
    }

    pub fn add_category(&self, path: &str) {
        let parts = path.split('/').collect::<Vec<&str>>();
        if parts.len() > 1 {
            if let Some(mut cat) = self.category_by_name("unreal") {
                let mut p = String::from("unreal");
                for (id, part) in parts.iter().enumerate() {
                    p.push('/');
                    p.push_str(part);
                    if id == parts.len() - 1 {
                        cat.add_category(part, &p, true);
                    } else {
                        cat.add_category(part, &p, false);
                        cat = self.add_category_by_name(part, &p);
                    }
                }
            }
        }
        if let Some(mut cat) = self.category_by_name("all") {
            let mut p = String::from("all");
            for (id, part) in parts.iter().enumerate() {
                p.push('/');
                p.push_str(part);
                if id == parts.len() - 1 {
                    cat.add_category(part, &p, true);
                } else {
                    cat.add_category(part, &p, false);
                    cat = self.add_category_by_name(part, &p);
                }
            }
        }
    }

    fn add_category_by_name(&self, part: &str, p: &str) -> EpicSidebarCategories {
        let self_ = self.imp();
        self.category_by_name(p).map_or_else(
            || {
                let c = categories::EpicSidebarCategories::new(
                    &categories::EpicSidebarCategories::capitalize_first_letter(part),
                    p,
                    Some(part),
                    Some(self),
                );
                c.set_widget_name(p);
                self_.stack.add_named(&c, Some(p));
                c
            },
            |c| c,
        )
    }
}

fn open_browser(code: &str) {
    #[cfg(target_os = "linux")]
    if gio::AppInfo::launch_default_for_uri(&format!("https://www.epicgames.com/id/exchange?exchangeCode={code}&redirectUrl=https%3A%2F%2Fwww.unrealengine.com%2Fmarketplace"), None::<&gio::AppLaunchContext>).is_err() {
        error!("Please go to https://www.epicgames.com/id/exchange?exchangeCode={code}&redirectUrl=https%3A%2F%2Fwww.unrealengine.com%2Fmarketplace");
    }
    #[cfg(target_os = "windows")]
    open::that(format!("https://www.epicgames.com/id/exchange?exchangeCode={code}&redirectUrl=https%3A%2F%2Fwww.unrealengine.com%2Fmarketplace"));
}
