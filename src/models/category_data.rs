use glib::ObjectExt;
use gtk4::{glib, subclass::prelude::*};

mod imp {
    use super::*;
    use gtk4::glib::ToValue;
    use std::cell::RefCell;

    #[derive(Debug, Default)]
    pub struct CategoryData {
        filter: RefCell<Option<String>>,
        name: RefCell<Option<String>>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for CategoryData {
        const NAME: &'static str = "CategoryData";
        type Type = super::CategoryData;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                filter: RefCell::new(None),
                name: RefCell::new(None),
            }
        }
    }

    impl ObjectImpl for CategoryData {
        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpec::new_string(
                        "name",
                        "Name",
                        "Name",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_string(
                        "filter",
                        "Filter",
                        "Filter",
                        None, // Default value
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
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "name" => {
                    let name = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.name.replace(name);
                }
                "filter" => {
                    let id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.filter.replace(id);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
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
    pub fn new(name: &str, filter: &str) -> CategoryData {
        let data: Self = glib::Object::new(&[("name", &name), ("filter", &filter)])
            .expect("Failed to create CategoryData");
        data
    }

    pub fn name(&self) -> String {
        if let Ok(value) = self.property("name") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        "".to_string()
    }

    pub fn filter(&self) -> String {
        if let Ok(value) = self.property("filter") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        "".to_string()
    }
}
