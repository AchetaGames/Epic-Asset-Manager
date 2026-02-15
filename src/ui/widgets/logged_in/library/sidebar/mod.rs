use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;
use log::error;
use std::thread;

pub mod button;
pub mod categories;
mod category;

pub mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use once_cell::sync::OnceCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/sidebar.ui")]
    pub struct EpicSidebar {
        pub actions: gio::SimpleActionGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub settings: gtk4::gio::Settings,
        pub loggedin: OnceCell<crate::ui::widgets::logged_in::library::EpicLibraryBox>,
        pub page_stack: OnceCell<gtk4::Stack>,
        #[template_child]
        pub engines_category: TemplateChild<button::EpicSidebarButton>,
        #[template_child]
        pub projects_category: TemplateChild<button::EpicSidebarButton>,
        #[template_child]
        pub library_category: TemplateChild<button::EpicSidebarButton>,
        #[template_child]
        pub fab_category: TemplateChild<button::EpicSidebarButton>,
        #[template_child]
        pub games_category: TemplateChild<button::EpicSidebarButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicSidebar {
        const NAME: &'static str = "EpicSidebar";
        type Type = super::EpicSidebar;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                actions: gio::SimpleActionGroup::new(),
                window: OnceCell::new(),
                loggedin: OnceCell::new(),
                page_stack: OnceCell::new(),
                engines_category: TemplateChild::default(),
                projects_category: TemplateChild::default(),
                library_category: TemplateChild::default(),
                fab_category: TemplateChild::default(),
                games_category: TemplateChild::default(),
                settings: gio::Settings::new(crate::config::APP_ID),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EpicSidebar {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            self.engines_category.set_sidebar(&obj);
            self.projects_category.set_sidebar(&obj);
            self.library_category.set_sidebar(&obj);
            self.fab_category.set_sidebar(&obj);
            self.games_category.set_sidebar(&obj);
        }
    }

    impl WidgetImpl for EpicSidebar {}
    impl BoxImpl for EpicSidebar {}
}

glib::wrapper! {
    pub struct EpicSidebar(ObjectSubclass<imp::EpicSidebar>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
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
        if self_.window.get().is_some() {
            return;
        }
        self_.window.set(window.clone()).unwrap();
    }

    pub fn set_logged_in(&self, loggedin: &crate::ui::widgets::logged_in::library::EpicLibraryBox) {
        let self_ = self.imp();
        if self_.loggedin.get().is_some() {
            return;
        }

        self_.loggedin.set(loggedin.clone()).unwrap();
        match self_.settings.string("default-category").as_str() {
            "engines" => &self_.engines_category,
            "projects" => &self_.projects_category,
            "fab" => &self_.fab_category,
            "games" => &self_.games_category,
            _ => &self_.library_category,
        }
        .clicked();
    }

    pub fn set_page_stack(&self, stack: &gtk4::Stack) {
        let self_ = self.imp();
        if self_.page_stack.get().is_some() {
            return;
        }
        self_.page_stack.set(stack.clone()).unwrap();
    }

    pub fn switch_main_page(&self, page: &str) {
        let self_ = self.imp();
        if let Some(stack) = self_.page_stack.get() {
            stack.set_visible_child_name(page);
        }
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("sidebar", Some(actions));

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

            thread::spawn(move || match crate::RUNTIME.block_on(eg.game_token()) {
                None => {}
                Some(token) => {
                    sender.send_blocking(token.code).unwrap();
                }
            });
        }
    }

    pub fn set_filter(&self, filter: Option<String>, path: Option<String>) {
        let self_ = self.imp();
        if let Some(p) = path {
            self.switch_main_page(&p);

            if let Some(l) = self_.loggedin.get() {
                l.set_property("filter", filter);
            }
        }
    }

    pub fn activate_all_buttons(&self) {
        let self_ = self.imp();
        self_.engines_category.activate(true);
        self_.projects_category.activate(true);
        self_.library_category.activate(true);
        self_.fab_category.activate(true);
        self_.games_category.activate(true);
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
