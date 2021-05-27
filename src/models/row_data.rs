use glib::ObjectExt;
use gtk::{glib, subclass::prelude::*};

// Implementation sub-module of the GObject
mod imp {
    use super::*;
    use glib::ToValue;
    use std::cell::RefCell;

    // The actual data structure that stores our values. This is not accessible
    // directly from the outside.
    #[derive(Debug, Default)]
    pub struct RowData {
        id: RefCell<Option<String>>,
        name: RefCell<Option<String>>,
        thumbnail: RefCell<Option<String>>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for RowData {
        const NAME: &'static str = "RowData";
        type Type = super::RowData;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                id: RefCell::new(None),
                name: RefCell::new(None),
                thumbnail: RefCell::new(None),
            }
        }
    }

    // The ObjectImpl trait provides the setters/getters for GObject properties.
    // Here we need to provide the values that are internally stored back to the
    // caller, or store whatever new value the caller is providing.
    //
    // This maps between the GObject properties and our internal storage of the
    // corresponding values of the properties.
    impl ObjectImpl for RowData {
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
                        "id",
                        "ID",
                        "ID",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_string(
                        "thumbnail",
                        "Thumbnail",
                        "Thumbnail",
                        None,
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
                "id" => {
                    let id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.id.replace(id);
                }
                "thumbnail" => {
                    let thumbnail = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.thumbnail.replace(thumbnail);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
                "id" => self.id.borrow().to_value(),
                "thumbnail" => self.thumbnail.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

// Public part of the RowData type. This behaves like a normal gtk-rs-style GObject
// binding
glib::wrapper! {
    pub struct RowData(ObjectSubclass<imp::RowData>);
}

// Constructor for new instances. This simply calls glib::Object::new() with
// initial values for our two properties and then returns the new instance
impl RowData {
    pub fn new(id: Option<String>, name: String, image: Vec<u8>) -> RowData {
        glib::Object::new(&[
            ("id", &id),
            ("name", &name),
            ("thumbnail", &Some(hex::encode(image))),
        ])
        .expect("Failed to create row data")
    }

    pub fn id(&self) -> String {
        if let Ok(value) = self.property("id") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        return "".to_string();
    }

    pub fn name(&self) -> String {
        if let Ok(value) = self.property("name") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        return "".to_string();
    }

    pub fn image(&self) -> Vec<u8> {
        match self.property("thumbnail") {
            Ok(i) => match i.get::<String>() {
                Ok(img) => match hex::decode(img) {
                    Ok(v) => v,
                    Err(_) => {
                        vec![]
                    }
                },
                Err(_) => {
                    vec![]
                }
            },
            Err(_) => {
                vec![]
            }
        }
    }
}
