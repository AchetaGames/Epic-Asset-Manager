use glib::ObjectExt;
use gtk4::{glib, subclass::prelude::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Uproject {
    pub file_version: i64,
    pub engine_association: String,
    pub category: String,
    pub description: String,
    pub modules: Option<Vec<Module>>,
    pub plugins: Option<Vec<Plugin>>,
    pub disable_engine_plugins_by_default: Option<bool>,
    pub enterprise: Option<bool>,
    pub additional_plugin_directories: Option<Vec<String>>,
    pub additional_root_directories: Option<Vec<String>>,
    pub target_platforms: Option<Vec<String>>,
    pub epic_sample_name_hash: Option<String>,
    pub pre_build_steps: Option<HashMap<String, Vec<String>>>,
    pub post_build_steps: Option<HashMap<String, Vec<String>>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Module {
    pub name: String,
    #[serde(rename = "Type")]
    pub type_field: String,
    pub loading_phase: String,
    pub additional_dependencies: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Plugin {
    pub name: String,
    pub enabled: bool,
    pub marketplace_url: Option<String>,
    pub supported_target_platforms: Option<Vec<String>>,
}

// Implementation sub-module of the GObject
mod imp {
    use super::*;
    use glib::ToValue;
    use gtk4::glib::ParamSpec;
    use std::cell::RefCell;

    // The actual data structure that stores our values. This is not accessible
    // directly from the outside.
    #[derive(Debug, Default)]
    pub struct ProjectData {
        guid: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        name: RefCell<Option<String>>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for ProjectData {
        const NAME: &'static str = "ProjectData";
        type Type = super::ProjectData;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                guid: RefCell::new(None),
                path: RefCell::new(None),
                name: RefCell::new(None),
            }
        }
    }

    // The ObjectImpl trait provides the setters/getters for GObject properties.
    // Here we need to provide the values that are internally stored back to the
    // caller, or store whatever new value the caller is providing.
    //
    // This maps between the GObject properties and our internal storage of the
    // corresponding values of the properties.
    impl ObjectImpl for ProjectData {
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
                        "name",
                        "Name",
                        "Name",
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
                "name" => {
                    let name = value.get().unwrap();
                    self.name.replace(name);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "guid" => self.guid.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "name" => self.name.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

// Public part of the ProjectData type. This behaves like a normal gtk-rs-style GObject
// binding
glib::wrapper! {
    pub struct ProjectData(ObjectSubclass<imp::ProjectData>);
}

// Constructor for new instances. This simply calls glib::Object::new() with
// initial values for our two properties and then returns the new instance
impl ProjectData {
    pub fn new(path: String, name: String) -> ProjectData {
        let data: Self = glib::Object::new(&[]).expect("Failed to create ProjectData");

        data.set_property("path", &path).unwrap();
        data.set_property("name", &name).unwrap();

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

    pub fn name(&self) -> String {
        if let Ok(value) = self.property("name") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        "".to_string()
    }
}
