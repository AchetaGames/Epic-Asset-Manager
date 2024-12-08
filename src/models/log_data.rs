use gtk4::prelude::ObjectExt;
use gtk4::{self, glib, subclass::prelude::*};

// Implementation sub-module of the GObject
mod imp {
    use super::*;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString};
    use gtk4::prelude::ToValue;
    use std::cell::RefCell;

    // The actual data structure that stores our values. This is not accessible
    // directly from the outside.
    #[derive(Debug)]
    pub struct LogData {
        path: RefCell<Option<String>>,
        name: RefCell<Option<String>>,
        crash: RefCell<bool>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for LogData {
        const NAME: &'static str = "LogData";
        type Type = super::LogData;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                path: RefCell::new(None),
                name: RefCell::new(None),
                crash: RefCell::new(false),
            }
        }
    }

    // The ObjectImpl trait provides the setters/getters for GObject properties.
    // Here we need to provide the values that are internally stored back to the
    // caller, or store whatever new value the caller is providing.
    //
    // This maps between the GObject properties and our internal storage of the
    // corresponding values of the properties.
    impl ObjectImpl for LogData {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("path").build(),
                    ParamSpecString::builder("name").build(),
                    ParamSpecBoolean::builder("crash").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "path" => {
                    let path = value.get().unwrap();
                    self.path.replace(path);
                }
                "name" => {
                    let name = value.get().unwrap();
                    self.name.replace(name);
                }
                "crash" => {
                    let crash = value.get().unwrap();
                    self.crash.replace(crash);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "path" => self.path.borrow().to_value(),
                "name" => self.name.borrow().to_value(),
                "crash" => self.crash.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

// Public part of the LogData type. This behaves like a normal gtk-rs-style GObject
// binding
glib::wrapper! {
    pub struct LogData(ObjectSubclass<imp::LogData>);
}

// Constructor for new instances. This simply calls glib::Object::new() with
// initial values for our two properties and then returns the new instance
impl LogData {
    pub fn new(path: &str, name: &str, crash: bool) -> LogData {
        let data: Self = glib::Object::new::<Self>();
        data.set_property("path", path);
        data.set_property("name", name);
        data.set_property("crash", crash);
        data
    }

    pub fn path(&self) -> Option<String> {
        self.property("path")
    }

    pub fn name(&self) -> Option<String> {
        self.property("name")
    }

    pub fn crash(&self) -> bool {
        self.property("crash")
    }
}
