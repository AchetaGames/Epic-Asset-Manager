use adw::gtk;
use glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{gio, glib, CompositeTemplate};
use gtk_macros::{action, get_action};

pub mod imp {
    use super::*;
    use crate::models::category_data::CategoryData;
    use glib::ParamSpec;
    use gtk4::glib::{ParamSpecBoolean, ParamSpecString};
    use gtk4::{gio, gio::ListStore, SingleSelection};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use std::collections::HashSet;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/sidebar_categories.ui")]
    pub struct EpicSidebarCategories {
        pub title: RefCell<Option<String>>,
        pub icon_name: RefCell<Option<String>>,
        pub path: RefCell<Option<String>>,
        pub filter: RefCell<Option<String>>,
        pub sidebar: OnceCell<crate::ui::widgets::logged_in::library::sidebar::EpicSidebar>,
        pub expanded: RefCell<bool>,
        pub actions: gio::SimpleActionGroup,
        #[template_child]
        pub categories: TemplateChild<gtk4::ListView>,
        #[template_child]
        pub previous: TemplateChild<gtk4::Button>,
        pub selection_model: SingleSelection,
        pub cats: ListStore,
        pub categories_set: RefCell<HashSet<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicSidebarCategories {
        const NAME: &'static str = "EpicSidebarCategories";
        type Type = super::EpicSidebarCategories;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                title: RefCell::new(None),
                icon_name: RefCell::new(None),
                path: RefCell::new(None),
                filter: RefCell::new(None),
                sidebar: OnceCell::new(),
                expanded: RefCell::new(false),
                actions: gio::SimpleActionGroup::new(),
                categories: TemplateChild::default(),
                cats: ListStore::new::<CategoryData>(),
                selection_model: SingleSelection::new(None::<gtk4::gio::ListModel>),
                categories_set: RefCell::new(HashSet::new()),
                previous: TemplateChild::default(),
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

    impl ObjectImpl for EpicSidebarCategories {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("title").build(),
                    ParamSpecString::builder("icon-name").build(),
                    ParamSpecString::builder("path").build(),
                    ParamSpecString::builder("filter").build(),
                    ParamSpecBoolean::builder("expanded").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "title" => {
                    let title = value.get().unwrap();
                    self.title.replace(title);
                }
                "path" => {
                    let path: Option<String> = value.get().unwrap();
                    self.path.replace(path);
                    self.obj().has_previous();
                }
                "filter" => {
                    let filter = value.get().unwrap();
                    self.filter.replace(filter);
                }
                "icon-name" => {
                    let icon_name = value.get().unwrap();
                    self.icon_name.replace(icon_name);
                }
                "expanded" => {
                    let expanded = value.get().unwrap();
                    self.expanded.replace(expanded);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "title" => self.title.borrow().to_value(),
                "icon-name" => self.icon_name.borrow().to_value(),
                "expanded" => self.expanded.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "filter" => self.filter.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            obj.setup_categories();
        }
    }

    impl WidgetImpl for EpicSidebarCategories {}
    impl BoxImpl for EpicSidebarCategories {}
}

glib::wrapper! {
    pub struct EpicSidebarCategories(ObjectSubclass<imp::EpicSidebarCategories>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicSidebarCategories {
    fn default() -> Self {
        Self::new("", "", None, None)
    }
}

impl EpicSidebarCategories {
    pub fn new(
        title: &str,
        path: &str,
        filter: Option<&str>,
        sidebar: Option<&crate::ui::widgets::logged_in::library::sidebar::EpicSidebar>,
    ) -> Self {
        let stack: Self = glib::Object::new();

        stack.set_property("title", title);
        stack.set_property("path", path);
        stack.set_property("filter", filter);
        if let Some(s) = sidebar {
            stack.set_sidebar(s);
        }

        stack
    }

    pub fn set_sidebar(
        &self,
        sidebar: &crate::ui::widgets::logged_in::library::sidebar::EpicSidebar,
    ) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.sidebar.get().is_some() {
            return;
        }

        self_.sidebar.set(sidebar.clone()).unwrap();
    }

    fn has_previous(&self) {
        let self_ = self.imp();
        if let Some(path) = self.path() {
            get_action!(self_.actions, @previous).set_enabled(path.contains('/'));
        }
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        self.insert_action_group("categories", Some(&self_.actions));
        action!(
            self_.actions,
            "previous",
            clone!(
                #[weak(rename_to=category)]
                self,
                move |_, _| {
                    category.back();
                }
            )
        );
        self.has_previous();
    }

    fn back(&self) {
        let self_ = self.imp();
        if let Some(s) = self_.sidebar.get() {
            if let Some(path) = self.path() {
                let mut parts = path.split('/').collect::<Vec<&str>>();
                parts.pop();
                s.set_filter(None, Some(parts.join("/")));
            }
        };
    }

    pub fn capitalize_first_letter(s: &str) -> String {
        let mut c = s.chars();
        c.next().map_or_else(String::new, |f| {
            f.to_uppercase().collect::<String>() + c.as_str()
        })
    }
    pub fn title(&self) -> Option<String> {
        self.property("title")
    }

    pub fn path(&self) -> Option<String> {
        self.property("path")
    }

    pub fn filter(&self) -> Option<String> {
        self.property("filter")
    }

    pub fn setup_categories(&self) {
        let self_ = self.imp();
        let factory = gtk4::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            item.set_child(Some(&crate::ui::widgets::logged_in::library::sidebar::category::EpicSidebarCategory::new()));
        });

        factory.connect_bind(move |_factory, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let data = list_item
                .item()
                .unwrap()
                .downcast::<crate::models::category_data::CategoryData>()
                .unwrap();

            let child = list_item.child().unwrap().downcast::<crate::ui::widgets::logged_in::library::sidebar::category::EpicSidebarCategory>().unwrap();
            child.set_property("title", data.name());
            child.set_property("leaf", data.leaf());
        });
        let sorter = gtk4::CustomSorter::new(move |obj1, obj2| {
            let info1 = obj1
                .downcast_ref::<crate::models::category_data::CategoryData>()
                .unwrap();
            let info2 = obj2
                .downcast_ref::<crate::models::category_data::CategoryData>()
                .unwrap();

            if info1.name().to_lowercase().eq("all") {
                gtk4::Ordering::Smaller
            } else if info2.name().to_lowercase().eq("all") {
                gtk4::Ordering::Larger
            } else if info1.name().to_lowercase().eq("downloaded") {
                gtk4::Ordering::Smaller
            } else if info2.name().to_lowercase().eq("downloaded") {
                gtk4::Ordering::Larger
            } else if info1.name().to_lowercase().eq("favorites") {
                gtk4::Ordering::Smaller
            } else if info2.name().to_lowercase().eq("favorites") {
                gtk4::Ordering::Larger
            } else {
                info1
                    .name()
                    .to_lowercase()
                    .cmp(&info2.name().to_lowercase())
                    .into()
            }
        });

        let sorted_model = gtk4::SortListModel::builder()
            .model(&self_.cats)
            .sorter(&sorter)
            .build();
        self_.selection_model.set_model(Some(&sorted_model));
        self_.selection_model.set_autoselect(false);
        self_.selection_model.set_can_unselect(true);
        self_.categories.set_model(Some(&self_.selection_model));
        self_.categories.set_factory(Some(&factory));

        self_.selection_model.connect_selected_notify(clone!(
            #[weak(rename_to=category)]
            self,
            move |model| {
                category.category_selected(model);
            }
        ));
    }

    fn category_selected(&self, model: &gtk4::SingleSelection) {
        let self_ = self.imp();

        if let Some(item) = model.selected_item() {
            model.unselect_item(model.selected());
            let filter = item
                .downcast::<crate::models::category_data::CategoryData>()
                .unwrap();
            if let Some(s) = self_.sidebar.get() {
                s.set_filter(Some(filter.filter()), Some(filter.path()));
            };
        }
    }

    pub fn add_category(&self, name: &str, path: &str, leaf: bool) {
        let self_ = self.imp();
        let mut cats = self_.categories_set.borrow_mut();
        if cats.insert(path.to_string()) {
            let category = crate::models::category_data::CategoryData::new(
                &Self::capitalize_first_letter(name),
                name,
                path,
                leaf,
            );
            self_.cats.append(&category);
        } else if !leaf {
            for i in 0..self_.cats.n_items() {
                let child = self_.cats.item(i).unwrap();
                let item = child
                    .downcast::<crate::models::category_data::CategoryData>()
                    .unwrap();
                if item.path().eq(path) {
                    item.set_property("leaf", false);
                    self_.cats.items_changed(i, 1, 1);
                    break;
                }
            }
        }
    }
}
