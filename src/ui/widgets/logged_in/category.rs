use crate::models::category_data::CategoryData;
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*, Label};
use gtk::{gio, glib, CompositeTemplate};
use gtk_macros::action;

pub(crate) mod imp {
    use super::*;
    use crate::models::category_data::CategoryData;
    use glib::ParamSpec;
    use gtk::{gio, gio::ListStore, SingleSelection};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/sidebar_category.ui")]
    pub struct EpicSidebarCategory {
        pub tooltip_text: RefCell<Option<String>>,
        pub icon_name: RefCell<Option<String>>,
        pub filter: RefCell<Option<String>>,
        pub loggedin: OnceCell<crate::ui::widgets::logged_in::EpicLoggedInBox>,
        pub expanded: RefCell<bool>,
        pub actions: gio::SimpleActionGroup,
        #[template_child]
        pub sub_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub sub_box: TemplateChild<gtk::ListView>,
        pub categories: ListStore,
        pub selection_model: SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicSidebarCategory {
        const NAME: &'static str = "EpicSidebarCategory";
        type Type = super::EpicSidebarCategory;
        type ParentType = gtk::Box;

        fn new() -> Self {
            Self {
                tooltip_text: RefCell::new(None),
                icon_name: RefCell::new(None),
                filter: RefCell::new(None),
                loggedin: OnceCell::new(),
                expanded: RefCell::new(false),
                actions: gio::SimpleActionGroup::new(),
                sub_revealer: TemplateChild::default(),
                sub_box: TemplateChild::default(),
                categories: ListStore::new(CategoryData::static_type()),
                selection_model: SingleSelection::new(None::<&gtk::SortListModel>),
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

    impl ObjectImpl for EpicSidebarCategory {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_string(
                        "tooltip-text",
                        "tooltip text",
                        "The category name",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "icon-name",
                        "icon name",
                        "The Icon Name",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "filter",
                        "Filter",
                        "Filter",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_boolean(
                        "expanded",
                        "expanded",
                        "Is expanded",
                        false,
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
                "tooltip-text" => {
                    let tooltip_text = value.get().unwrap();
                    self.tooltip_text.replace(tooltip_text);
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
                    if expanded {
                        self.sub_revealer.set_visible(true);
                    } else {
                        self.sub_revealer.set_visible(false);
                    }
                    self.expanded.replace(expanded);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "tooltip-text" => self.tooltip_text.borrow().to_value(),
                "icon-name" => self.icon_name.borrow().to_value(),
                "expanded" => self.expanded.borrow().to_value(),
                "filter" => self.filter.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
            obj.setup_categories();
        }
    }

    impl WidgetImpl for EpicSidebarCategory {}
    impl BoxImpl for EpicSidebarCategory {}
}

glib::wrapper! {
    pub struct EpicSidebarCategory(ObjectSubclass<imp::EpicSidebarCategory>)
        @extends gtk::Widget, gtk::Box;
}

impl EpicSidebarCategory {
    pub fn new() -> Self {
        let stack: Self = glib::Object::new(&[]).expect("Failed to create EpicSidebarCategory");

        stack
    }

    pub fn set_logged_in(&self, loggedin: &crate::ui::widgets::logged_in::EpicLoggedInBox) {
        let self_: &imp::EpicSidebarCategory = imp::EpicSidebarCategory::from_instance(self);
        // Do not run this twice
        if let Some(_) = self_.loggedin.get() {
            return;
        }

        self_.loggedin.set(loggedin.clone()).unwrap();
    }

    pub fn setup_categories(&self) {
        let self_: &imp::EpicSidebarCategory = imp::EpicSidebarCategory::from_instance(self);
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let row = Label::new(None);
            row.set_halign(gtk::Align::Fill);
            row.set_xalign(0.0);
            item.set_child(Some(&row));
        });

        factory.connect_bind(move |_factory, list_item| {
            let data = list_item
                .item()
                .unwrap()
                .downcast::<crate::models::category_data::CategoryData>()
                .unwrap();

            let child = list_item.child().unwrap().downcast::<Label>().unwrap();
            child.set_label(&data.name());
            child.set_tooltip_text(Some(&data.filter()))
        });
        let sorter = gtk::CustomSorter::new(move |obj1, obj2| {
            let info1 = obj1
                .downcast_ref::<crate::models::category_data::CategoryData>()
                .unwrap();
            let info2 = obj2
                .downcast_ref::<crate::models::category_data::CategoryData>()
                .unwrap();

            info1
                .name()
                .to_lowercase()
                .cmp(&info2.name().to_lowercase())
                .into()
        });

        let sorted_model = gtk::SortListModel::new(Some(&self_.categories), Some(&sorter));
        self_.selection_model.set_model(Some(&sorted_model));
        self_.selection_model.set_autoselect(false);
        self_.selection_model.set_can_unselect(true);
        self_.sub_box.set_model(Some(&self_.selection_model));
        self_.sub_box.set_factory(Some(&factory));

        self_.selection_model.connect_selected_notify(clone!(@weak self as category => move |model| {
            let self_: &imp::EpicSidebarCategory = imp::EpicSidebarCategory::from_instance(&category);
            if let Some(item) = model.selected_item() {
                let filter = item.downcast::<crate::models::category_data::CategoryData>().unwrap();
                if let Some(l) = self_.loggedin.get() {
                    l.set_property("filter", filter.filter()).unwrap();
                };
            }
        }));
    }

    pub fn setup_actions(&self) {
        let self_: &imp::EpicSidebarCategory = imp::EpicSidebarCategory::from_instance(self);
        action!(
            self_.actions,
            "clicked",
            clone!(@weak self as win => move |_, _| {
                if let Ok(v) = win.property("expanded") {
                    let self_: &imp::EpicSidebarCategory = imp::EpicSidebarCategory::from_instance(&win);
                    if v.get::<bool>().unwrap() {
                        if self_.sub_box.first_child().is_none() {
                            if let Some(l) = self_.loggedin.get() { l.set_property("filter", win.filter()).unwrap(); };
                        } else {
                            self_.sub_revealer.set_reveal_child(!self_.sub_revealer.reveals_child());
                            }
                    } else {
                        if let Some(l) = self_.loggedin.get() { l.set_property("filter", win.filter()).unwrap(); };
                    }
                }

            })
        );
        self.insert_action_group("category", Some(&self_.actions));
    }

    fn capitalize_first_letter(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }

    pub fn add_category(&self, name: String, filter: String) {
        let self_: &imp::EpicSidebarCategory = imp::EpicSidebarCategory::from_instance(self);
        self_.categories.append(&CategoryData::new(
            EpicSidebarCategory::capitalize_first_letter(&name),
            filter,
        ))
    }

    pub fn filter(&self) -> String {
        if let Ok(value) = self.property("filter") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        return "".to_string();
    }

    pub fn unselect_except(&self, category: &String) {
        let self_: &imp::EpicSidebarCategory = imp::EpicSidebarCategory::from_instance(self);
        let selected_id = self_.selection_model.selected();
        if let Some(item) = self_.selection_model.selected_item() {
            let cat = item
                .downcast::<crate::models::category_data::CategoryData>()
                .unwrap();
            if !cat.filter().eq(category) {
                self_.selection_model.unselect_item(selected_id);
            }
        }
    }
}
