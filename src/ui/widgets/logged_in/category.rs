use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{gio, glib, CompositeTemplate};
use gtk_macros::action;

pub(crate) mod imp {
    use super::*;
    use glib::ParamSpec;
    use gtk::gio;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/sidebar_category.ui")]
    pub struct EpicSidebarCategory {
        pub tooltip_text: RefCell<Option<String>>,
        pub icon_name: RefCell<Option<String>>,
        pub expanded: RefCell<bool>,
        pub actions: gio::SimpleActionGroup,
        #[template_child]
        pub sub_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub sub_box: TemplateChild<gtk::Box>,
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
                expanded: RefCell::new(false),
                actions: gio::SimpleActionGroup::new(),
                sub_revealer: TemplateChild::default(),
                sub_box: TemplateChild::default(),
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
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
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
                            println!("Clicked empty expanded");
                        } else {
                            self_.sub_revealer.set_reveal_child(!self_.sub_revealer.reveals_child());
                            }
                    } else {
                        println!("Clicked collapsed");
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
        let label = gtk::LabelBuilder::new()
            .label(&EpicSidebarCategory::capitalize_first_letter(&name))
            .halign(gtk::Align::Start)
            .build();
        let button = gtk::ButtonBuilder::new()
            .name("subcategory")
            .child(&label)
            .build();

        button.connect_clicked(clone!(@weak self as win => move |_| {
            println!("Subclicked {}", filter);
        }));
        button.show();
        self_.sub_box.append(&button);
    }
}
