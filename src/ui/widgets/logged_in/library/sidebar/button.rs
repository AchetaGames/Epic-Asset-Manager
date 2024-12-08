use glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{gio, glib, CompositeTemplate};
use gtk_macros::action;

pub mod imp {
    use super::*;
    use crate::models::category_data::CategoryData;
    use glib::ParamSpec;
    use gtk4::glib::{ParamSpecBoolean, ParamSpecString};
    use gtk4::{gio, gio::ListStore};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/sidebar_button.ui")]
    pub struct EpicSidebarButton {
        pub tooltip_text: RefCell<Option<String>>,
        pub icon_name: RefCell<Option<String>>,
        pub filter: RefCell<Option<String>>,
        pub path: RefCell<Option<String>>,
        pub sidebar: OnceCell<crate::ui::widgets::logged_in::library::sidebar::EpicSidebar>,
        pub expanded: RefCell<bool>,
        pub actions: gio::SimpleActionGroup,
        #[template_child]
        pub category_button: TemplateChild<gtk4::Button>,
        #[template_child]
        pub separator: TemplateChild<gtk4::Separator>,
        pub categories: ListStore,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicSidebarButton {
        const NAME: &'static str = "EpicSidebarButton";
        type Type = super::EpicSidebarButton;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                tooltip_text: RefCell::new(None),
                icon_name: RefCell::new(None),
                filter: RefCell::new(None),
                path: RefCell::new(None),
                sidebar: OnceCell::new(),
                expanded: RefCell::new(false),
                actions: gio::SimpleActionGroup::new(),
                category_button: TemplateChild::default(),
                separator: TemplateChild::default(),
                categories: ListStore::new::<CategoryData>(),
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

    impl ObjectImpl for EpicSidebarButton {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("tooltip-text").build(),
                    ParamSpecString::builder("path").build(),
                    ParamSpecString::builder("icon-name").build(),
                    ParamSpecString::builder("filter").build(),
                    ParamSpecBoolean::builder("expanded").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "tooltip-text" => {
                    let tooltip_text = value.get().unwrap();
                    self.tooltip_text.replace(tooltip_text);
                }
                "filter" => {
                    let filter = value.get().unwrap();
                    self.filter.replace(filter);
                }
                "path" => {
                    let path = value.get().unwrap();
                    self.path.replace(path);
                }
                "icon-name" => {
                    let icon_name = value.get().unwrap();
                    self.icon_name.replace(icon_name);
                }
                "expanded" => {
                    let expanded = value.get().unwrap();
                    self.expanded.replace(expanded);
                    self.separator
                        .set_visible(expanded && self.category_button.is_sensitive());
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "tooltip-text" => self.tooltip_text.borrow().to_value(),
                "icon-name" => self.icon_name.borrow().to_value(),
                "expanded" => self.expanded.borrow().to_value(),
                "filter" => self.filter.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_actions();
        }
    }

    impl WidgetImpl for EpicSidebarButton {}
    impl BoxImpl for EpicSidebarButton {}
}

glib::wrapper! {
    pub struct EpicSidebarButton(ObjectSubclass<imp::EpicSidebarButton>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicSidebarButton {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicSidebarButton {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_sidebar(
        &self,
        loggedin: &crate::ui::widgets::logged_in::library::sidebar::EpicSidebar,
    ) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.sidebar.get().is_some() {
            return;
        }

        self_.sidebar.set(loggedin.clone()).unwrap();
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        self.insert_action_group("sidebar_button", Some(&self_.actions));
        action!(
            self_.actions,
            "clicked",
            clone!(
                #[weak(rename_to=button)]
                self,
                move |_, _| {
                    button.clicked();
                }
            )
        );
    }

    pub fn clicked(&self) {
        let self_ = self.imp();

        if let Some(s) = self_.sidebar.get() {
            s.set_filter(self.filter(), self.path());
            s.activate_all_buttons();
        }

        self.activate(false);
    }

    pub fn filter(&self) -> Option<String> {
        self.property("filter")
    }
    pub fn path(&self) -> Option<String> {
        self.property("path")
    }

    pub fn activate(&self, activate: bool) {
        let self_ = self.imp();
        if activate {
            self_.category_button.add_css_class("flat");
        } else {
            self_.category_button.remove_css_class("flat");
        }
        self_.separator.set_visible(activate && self.expanded());
    }

    pub fn expanded(&self) -> bool {
        self.property("expanded")
    }
}
