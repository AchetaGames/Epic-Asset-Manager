use glib::ObjectExt;
use gtk4::{glib, subclass::prelude::*};

// Implementation sub-module of the GObject
mod imp {
    use super::*;
    use glib::ToValue;
    use gtk4::glib::ParamSpec;
    use std::cell::RefCell;

    // The actual data structure that stores our values. This is not accessible
    // directly from the outside.
    #[derive(Debug, Default)]
    pub struct EngineData {
        guid: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        version: RefCell<Option<String>>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for EngineData {
        const NAME: &'static str = "EngineData";
        type Type = super::EngineData;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                guid: RefCell::new(None),
                path: RefCell::new(None),
                version: RefCell::new(None),
            }
        }
    }

    // The ObjectImpl trait provides the setters/getters for GObject properties.
    // Here we need to provide the values that are internally stored back to the
    // caller, or store whatever new value the caller is providing.
    //
    // This maps between the GObject properties and our internal storage of the
    // corresponding values of the properties.
    impl ObjectImpl for EngineData {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_string(
                        "guid",
                        "GUID",
                        "GUID",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "path",
                        "Path",
                        "Path",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "version",
                        "Version",
                        "Version",
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
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "guid" => {
                    let guid = value.get().unwrap();
                    self.guid.replace(guid);
                }
                "path" => {
                    let path = value.get().unwrap();
                    self.path.replace(path);
                }
                "version" => {
                    let version = value.get().unwrap();
                    self.version.replace(version);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "guid" => self.guid.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "version" => self.version.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

// Public part of the EngineData type. This behaves like a normal gtk-rs-style GObject
// binding
glib::wrapper! {
    pub struct EngineData(ObjectSubclass<imp::EngineData>);
}

// Constructor for new instances. This simply calls glib::Object::new() with
// initial values for our two properties and then returns the new instance
impl EngineData {
    pub fn new(
        path: String,
        guid: String,
        version: crate::ui::widgets::logged_in::engine::UnrealVersion,
    ) -> EngineData {
        let data: Self = glib::Object::new(&[]).expect("Failed to create EngineData");

        data.set_property("path", &path).unwrap();
        data.set_property("guid", &guid).unwrap();
        data.set_property(
            "version",
            format!(
                "{}.{}.{}",
                version.major_version, version.minor_version, version.patch_version
            ),
        )
        .unwrap();

        data
    }

    pub fn guid(&self) -> String {
        if let Ok(value) = self.property("guid") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        "".to_string()
    }

    pub fn path(&self) -> String {
        if let Ok(value) = self.property("path") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        "".to_string()
    }

    pub fn version(&self) -> String {
        if let Ok(value) = self.property("version") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        "".to_string()
    }
}
