use gtk4::prelude::ObjectExt;
use gtk4::{self, glib, subclass::prelude::*};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Uplugin {
    #[serde(default)]
    pub file_version: i64,
    #[serde(default)]
    pub version: i64,
    #[serde(default)]
    pub version_name: String,
    #[serde(default)]
    pub friendly_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub created_by: String,
    #[serde(default)]
    pub docs_url: String,
    #[serde(default)]
    pub marketplace_url: String,
    #[serde(default)]
    pub support_url: String,
    pub engine_version: Option<Vec<String>>,
    pub editor_custom_virtual_path: Option<Vec<String>>,
    pub enabled_by_default: Option<bool>,
    pub can_contain_content: Option<bool>,
    pub can_contain_verse: Option<bool>,
    pub is_beta_version: Option<bool>,
    pub is_experimental_version: Option<bool>,
    pub installed: Option<bool>,
    pub supported_target_platforms: Option<Vec<String>>,
    pub supported_programs: Option<Vec<String>>,
    pub b_is_plugin_extension: Option<bool>,
    pub hidden: Option<bool>,
    pub explicitly_loaded: Option<bool>,
    pub has_explicit_platforms: Option<bool>,
    pub pre_build_steps: Option<HashMap<String, Vec<String>>>,
    pub post_build_steps: Option<HashMap<String, Vec<String>>>,
    pub plugins: Option<Vec<Plugin>>,
    pub modules: Option<Vec<Module>>,
    pub editor_only: Option<bool>,
    pub is_hidden: Option<bool>,
    pub is_experimental: Option<bool>,
    #[serde(default)]
    pub localization_targets: Vec<HashMap<String, String>>,
    pub requires_build_platform: Option<bool>,
    pub can_be_used_with_unreal_header_tool: Option<bool>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Module {
    pub name: String,
    #[serde(rename = "Type", default)]
    pub type_field: String,
    #[serde(default)]
    pub loading_phase: String,
    pub additional_dependencies: Option<Vec<String>>,
    #[serde(default)]
    pub platform_allow_list: Vec<String>,
    #[serde(default)]
    pub program_allow_list: Vec<String>,
    #[serde(default)]
    pub target_deny_list: Vec<String>,
    #[serde(default)]
    pub platform_deny_list: Vec<String>,
    #[serde(default)]
    pub target_configuration_deny_list: Vec<String>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Plugin {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub marketplace_url: Option<String>,
    #[serde(default)]
    pub supported_target_platforms: Option<Vec<String>>,
    #[serde(default)]
    pub platform_allow_list: Vec<String>,
    #[serde(default)]
    pub target_allow_list: Vec<String>,
    #[serde(default)]
    pub target_deny_list: Vec<String>,
    pub optional: Option<bool>,
    #[serde(default)]
    pub platform_deny_list: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {}

// Implementation sub-module of the GObject
mod imp {
    use super::*;
    use gtk4::glib::{ParamSpec, ParamSpecString};
    use gtk4::prelude::ToValue;
    use std::cell::RefCell;

    // The actual data structure that stores our values. This is not accessible
    // directly from the outside.
    #[derive(Debug)]
    pub struct PluginData {
        guid: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        name: RefCell<Option<String>>,
        pub uplugin: RefCell<Option<Uplugin>>,
        pub sender: async_channel::Sender<Msg>,
        pub receiver: RefCell<Option<async_channel::Receiver<Msg>>>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for PluginData {
        const NAME: &'static str = "PluginData";
        type Type = super::PluginData;
        type ParentType = glib::Object;

        fn new() -> Self {
            let (sender, receiver) = async_channel::unbounded();
            Self {
                guid: RefCell::new(None),
                path: RefCell::new(None),
                name: RefCell::new(None),
                uplugin: RefCell::new(None),
                sender,
                receiver: RefCell::new(Some(receiver)),
            }
        }
    }

    // The ObjectImpl trait provides the setters/getters for GObject properties.
    // Here we need to provide the values that are internally stored back to the
    // caller, or store whatever new value the caller is providing.
    //
    // This maps between the GObject properties and our internal storage of the
    // corresponding values of the properties.
    impl ObjectImpl for PluginData {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_messaging();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder("finished")
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("guid").build(),
                    ParamSpecString::builder("path").build(),
                    ParamSpecString::builder("name").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
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

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "guid" => self.guid.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "name" => self.name.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

// Public part of the PluginData type. This behaves like a normal gtk-rs-style GObject
// binding
glib::wrapper! {
    pub struct PluginData(ObjectSubclass<imp::PluginData>);
}

// Constructor for new instances. This simply calls glib::Object::new() with
// initial values for our two properties and then returns the new instance
impl PluginData {
    pub fn new(path: &str, name: &str) -> PluginData {
        let data: Self = glib::Object::new();
        data.set_property("path", path);
        data.set_property("name", name);
        data
    }

    pub fn guid(&self) -> Option<String> {
        self.property("guid")
    }

    pub fn path(&self) -> Option<String> {
        self.property("path")
    }

    pub fn name(&self) -> Option<String> {
        self.property("name")
    }

    pub fn read_uplugin(path: &str) -> Uplugin {
        let p = std::path::PathBuf::from(path);
        if let Ok(mut file) = File::open(p) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                return serde_json::from_str(&contents).unwrap();
            }
        }
        Uplugin::default()
    }

    pub fn uplugin(&self) -> Option<Uplugin> {
        let self_ = self.imp();
        self_.uplugin.borrow().clone()
    }

    pub fn setup_messaging(&self) {
        let self_ = self.imp();
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        let project = self.clone();
        glib::spawn_future_local(async move {
            while let Ok(response) = receiver.recv().await {
                project.update(response);
            }
        });
    }

    pub fn update(&self, _msg: Msg) {
        debug!("Update for {:?}", self.name());
    }
}
