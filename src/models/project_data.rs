use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::thread;

use glib::ObjectExt;
use gtk4::gdk::Texture;
use gtk4::{self, glib, subclass::prelude::*};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Uproject {
    #[serde(default)]
    pub file_version: i64,
    #[serde(default)]
    pub engine_association: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    pub modules: Option<Vec<super::plugin_data::Module>>,
    pub plugins: Option<Vec<super::plugin_data::Plugin>>,
    pub disable_engine_plugins_by_default: Option<bool>,
    pub enterprise: Option<bool>,
    pub additional_plugin_directories: Option<Vec<String>>,
    pub additional_root_directories: Option<Vec<String>>,
    pub target_platforms: Option<Vec<String>>,
    pub epic_sample_name_hash: Option<String>,
    pub pre_build_steps: Option<HashMap<String, Vec<String>>>,
    pub post_build_steps: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Thumbnail(Texture),
}

// Implementation sub-module of the GObject
mod imp {
    use std::cell::RefCell;

    use glib::ToValue;
    use gtk4::gdk::Texture;
    use gtk4::glib::{ParamSpec, ParamSpecObject, ParamSpecString};

    use super::*;

    // The actual data structure that stores our values. This is not accessible
    // directly from the outside.
    #[derive(Debug)]
    pub struct ProjectData {
        guid: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        name: RefCell<Option<String>>,
        pub uproject: RefCell<Option<Uproject>>,
        thumbnail: RefCell<Option<Texture>>,
        pub sender: async_channel::Sender<Msg>,
        pub receiver: RefCell<Option<async_channel::Receiver<Msg>>>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for ProjectData {
        const NAME: &'static str = "ProjectData";
        type Type = super::ProjectData;
        type ParentType = glib::Object;

        fn new() -> Self {
            let (sender, receiver) = async_channel::bounded(1);
            Self {
                guid: RefCell::new(None),
                path: RefCell::new(None),
                name: RefCell::new(None),
                uproject: RefCell::new(None),
                thumbnail: RefCell::new(None),
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
    impl ObjectImpl for ProjectData {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_messaging();
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![glib::subclass::Signal::builder("finished")
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
                    ParamSpecObject::builder::<Texture>("thumbnail").build(),
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
                "thumbnail" => {
                    let thumbnail = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.thumbnail.replace(thumbnail);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "guid" => self.guid.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "name" => self.name.borrow().to_value(),
                "thumbnail" => self.thumbnail.borrow().to_value(),
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
    pub fn new(path: &str, name: &str) -> ProjectData {
        let data: Self = glib::Object::new::<Self>();
        let self_ = data.imp();
        data.set_property("path", path);
        data.set_property("name", name);
        let mut uproject = Self::read_uproject(path);
        uproject.engine_association = uproject
            .engine_association
            .chars()
            .filter(|c| c != &'{' && c != &'}')
            .collect();
        self_.uproject.replace(Some(uproject));
        if let Some(path) = data.path() {
            let sender = self_.sender.clone();
            thread::spawn(move || {
                Self::load_thumbnail(&path, &sender);
            });
        }
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

    pub fn image(&self) -> Option<Texture> {
        self.property("thumbnail")
    }

    pub fn read_uproject(path: &str) -> Uproject {
        let p = std::path::PathBuf::from(path);
        if let Ok(mut file) = std::fs::File::open(p) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                return serde_json::from_str::<Uproject>(&contents).unwrap_or_else(|e| {
                    error!("Unable to parse uproject {path}: {e}");
                    Uproject::default()
                });
            }
        }
        Uproject::default()
    }

    pub fn uproject(&self) -> Option<Uproject> {
        let self_ = self.imp();
        self_.uproject.borrow().clone()
    }

    pub fn setup_messaging(&self) {
        let self_ = self.imp();
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        let project = self.clone();
        glib::spawn_future_local(async move {
            while let Ok(response) = receiver.recv().await {
                debug!("project_data: {:?}", &response);
                project.update(response);
            }
        });
    }

    pub fn update(&self, msg: Msg) {
        match msg {
            Msg::Thumbnail(image) => {
                self.set_property("thumbnail", Some(image));
            }
        };
        self.emit_by_name::<()>("finished", &[]);
    }

    pub fn load_thumbnail(path: &str, sender: &async_channel::Sender<Msg>) {
        let mut pathbuf = match PathBuf::from(&path).parent() {
            None => return,
            Some(p) => p.to_path_buf(),
        };
        pathbuf.push("Saved");
        pathbuf.push("AutoScreenshot.png");
        if pathbuf.exists() {
            match Texture::from_file(&gtk4::gio::File::for_path(pathbuf.as_path())) {
                Ok(t) => sender.send_blocking(Msg::Thumbnail(t)).unwrap(),
                Err(e) => {
                    error!("Unable to load file to texture: {}", e);
                }
            };
        } else {
            info!("No project picture exists for {}", path);
        }
    }
}
