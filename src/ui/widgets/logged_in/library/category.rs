use crate::models::category_data::CategoryData;
use glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*, Label};
use gtk4::{gio, glib, CompositeTemplate};
use gtk_macros::action;

pub(crate) mod imp {
    use super::*;
    use crate::models::category_data::CategoryData;
    use glib::ParamSpec;
    use gtk4::glib::{ParamSpecBoolean, ParamSpecString};
    use gtk4::{gio, gio::ListStore, SingleSelection};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/sidebar_category.ui")]
    pub struct EpicSidebarCategory {
        pub tooltip_text: RefCell<Option<String>>,
        pub icon_name: RefCell<Option<String>>,
        pub filter: RefCell<Option<String>>,
        pub loggedin: OnceCell<crate::ui::widgets::logged_in::library::EpicLibraryBox>,
        pub expanded: RefCell<bool>,
        pub actions: gio::SimpleActionGroup,
        #[template_child]
        pub sub_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub sub_box: TemplateChild<gtk4::ListView>,
        #[template_child]
        pub category_button: TemplateChild<gtk4::Button>,
        pub categories: ListStore,
        pub selection_model: SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicSidebarCategory {
        const NAME: &'static str = "EpicSidebarCategory";
        type Type = super::EpicSidebarCategory;
        type ParentType = gtk4::Box;

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
                category_button: TemplateChild::default(),
                categories: ListStore::new(CategoryData::static_type()),
                selection_model: SingleSelection::new(None::<&gtk4::SortListModel>),
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
                    ParamSpecString::new(
                        "tooltip-text",
                        "tooltip text",
                        "The category name",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecString::new(
                        "icon-name",
                        "icon name",
                        "The Icon Name",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecString::new(
                        "filter",
                        "Filter",
                        "Filter",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecBoolean::new(
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
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicSidebarCategory {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicSidebarCategory {
    pub fn new() -> Self {
        let stack: Self = glib::Object::new(&[]).expect("Failed to create EpicSidebarCategory");

        stack
    }

    pub fn set_logged_in(&self, loggedin: &crate::ui::widgets::logged_in::library::EpicLibraryBox) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.loggedin.get().is_some() {
            return;
        }

        self_.loggedin.set(loggedin.clone()).unwrap();
    }

    pub fn setup_categories(&self) {
        let self_ = self.imp();
        let factory = gtk4::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let row = Label::new(None);
            row.set_halign(gtk4::Align::Fill);
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
            child.set_tooltip_text(Some(&data.filter()));
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

        let sorted_model = gtk4::SortListModel::new(Some(&self_.categories), Some(&sorter));
        self_.selection_model.set_model(Some(&sorted_model));
        self_.selection_model.set_autoselect(false);
        self_.selection_model.set_can_unselect(true);
        self_.sub_box.set_model(Some(&self_.selection_model));
        self_.sub_box.set_factory(Some(&factory));

        self_.selection_model.connect_selected_notify(
            clone!(@weak self as category => move |model| {
                category.category_selected(model);
            }),
        );
    }

    fn category_selected(&self, model: &gtk4::SingleSelection) {
        let self_ = self.imp();
        if let Some(item) = model.selected_item() {
            let filter = item
                .downcast::<crate::models::category_data::CategoryData>()
                .unwrap();
            if let Some(l) = self_.loggedin.get() {
                l.set_property("filter", filter.filter());
            };
        }
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        action!(
            self_.actions,
            "clicked",
            clone!(@weak self as category => move |_, _| {
                category.clicked();
            })
        );
        self.insert_action_group("category", Some(&self_.actions));
    }

    pub fn clicked(&self) {
        let v: glib::Value = self.property("expanded");
        let self_ = self.imp();
        if v.get::<bool>().unwrap() {
            if self_.sub_box.first_child().is_none() {
                if let Some(l) = self_.loggedin.get() {
                    l.set_property("filter", self.filter());
                };
            } else {
                self_
                    .sub_revealer
                    .set_reveal_child(!self_.sub_revealer.reveals_child());
            }
        } else if let Some(l) = self_.loggedin.get() {
            l.enable_all_categories();
            l.set_property("filter", self.filter());
            self.activate(false);
        };
    }

    fn capitalize_first_letter(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }

    pub fn add_category(&self, name: &str, filter: &str) {
        let self_ = self.imp();
        self_.categories.append(&CategoryData::new(
            &EpicSidebarCategory::capitalize_first_letter(name),
            filter,
        ));
    }

    pub fn filter(&self) -> Option<String> {
        self.property("filter")
    }

    pub fn unselect_except(&self, category: &str) {
        let self_ = self.imp();
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

    pub fn activate(&self, activate: bool) {
        let self_ = self.imp();
        self_.category_button.set_sensitive(activate);
    }
}
