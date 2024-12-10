use gtk4::prelude::ObjectExt;
use gtk4::{glib, subclass::prelude::*};

mod imp {
    use super::*;
    use gtk4::prelude::ToValue;
    use std::cell::RefCell;

    #[derive(Debug, Default)]
    pub struct CategoryData {
        filter: RefCell<Option<String>>,
        name: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        leaf: RefCell<bool>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for CategoryData {
        const NAME: &'static str = "CategoryData";
        type Type = super::CategoryData;

        fn new() -> Self {
            Self {
                filter: RefCell::new(None),
                name: RefCell::new(None),
                path: RefCell::new(None),
                leaf: RefCell::new(false),
            }
        }
    }

    impl ObjectImpl for CategoryData {
        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("name").build(),
                    glib::ParamSpecString::builder("path").build(),
                    glib::ParamSpecString::builder("filter").build(),
                    glib::ParamSpecBoolean::builder("leaf").build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "name" => {
                    let name = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.name.replace(name);
                }
                "path" => {
                    let path = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.path.replace(path);
                }
                "filter" => {
                    let id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.filter.replace(id);
                }
                "leaf" => {
                    let id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.leaf.replace(id);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "leaf" => self.leaf.borrow().to_value(),
                "filter" => self.filter.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct CategoryData(ObjectSubclass<imp::CategoryData>);
}

impl CategoryData {
    pub fn new(name: &str, filter: &str, path: &str, leaf: bool) -> CategoryData {
        glib::Object::builder()
            .property("name", name)
            .property("filter", filter)
            .property("path", path)
            .property("leaf", leaf)
            .build()
    }

    pub fn name(&self) -> String {
        self.property("name")
    }
    pub fn path(&self) -> String {
        self.property("path")
    }

    pub fn filter(&self) -> String {
        self.property("filter")
    }

    pub fn leaf(&self) -> bool {
        self.property("leaf")
    }
}
