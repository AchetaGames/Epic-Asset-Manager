use std::collections::HashMap;
use std::ffi::OsString;
use std::str::FromStr;

use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;
use log::{debug, error, warn};

use engine::EpicEngine;
use version_compare::Cmp;

pub mod engine;
pub mod engine_detail;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct UnrealEngine {
    pub version: crate::models::engine_data::UnrealVersion,
    pub path: String,
    pub guid: Option<String>,
}

impl UnrealEngine {
    pub fn get_engine_binary_path(&self) -> Option<OsString> {
        if let Ok(mut p) = std::path::PathBuf::from_str(&self.path) {
            p.push("Engine");
            p.push("Binaries");
            p.push("Linux");
            let mut test = p.clone();
            test.push("UE4Editor");
            if test.exists() {
                let mut result = OsString::new();
                result.push(test.into_os_string());
                return Some(result);
            }
            let mut test = p.clone();
            test.push("UnrealEditor");
            if test.exists() {
                let mut result = OsString::new();
                result.push(test.into_os_string());
                return Some(result);
            }
            error!("Unable to launch the engine");
        };
        None
    }
}

pub(crate) mod imp {
    use std::cell::RefCell;

    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString};
    use once_cell::sync::OnceCell;

    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/engines.ui")]
    pub struct EpicEnginesBox {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        #[template_child]
        pub engine_grid: TemplateChild<gtk4::GridView>,
        #[template_child]
        pub details:
            TemplateChild<crate::ui::widgets::logged_in::engines::engine_detail::EpicEngineDetails>,
        pub grid_model: gtk4::gio::ListStore,
        pub expanded: RefCell<bool>,
        selected: RefCell<Option<String>>,
        pub actions: gtk4::gio::SimpleActionGroup,
        pub engines: RefCell<HashMap<String, super::UnrealEngine>>,
        pub settings: gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEnginesBox {
        const NAME: &'static str = "EpicEnginesBox";
        type Type = super::EpicEnginesBox;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                engine_grid: TemplateChild::default(),
                details: TemplateChild::default(),
                grid_model: gtk4::gio::ListStore::new(
                    crate::models::engine_data::EngineData::static_type(),
                ),
                expanded: RefCell::new(false),
                selected: RefCell::new(None),
                actions: gtk4::gio::SimpleActionGroup::new(),
                engines: RefCell::new(HashMap::new()),
                settings: gio::Settings::new(crate::config::APP_ID),
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

    impl ObjectImpl for EpicEnginesBox {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::new(
                        "expanded",
                        "expanded",
                        "Is expanded",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecString::new(
                        "selected",
                        "Selected",
                        "Selected",
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
                "expanded" => {
                    let expanded = value.get().unwrap();
                    self.expanded.replace(expanded);
                }
                "selected" => {
                    let selected = value.get().unwrap();
                    self.selected.replace(selected);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "expanded" => self.expanded.borrow().to_value(),
                "selected" => self.selected.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicEnginesBox {}
    impl BoxImpl for EpicEnginesBox {}
}

glib::wrapper! {
    pub struct EpicEnginesBox(ObjectSubclass<imp::EpicEnginesBox>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicEnginesBox {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicEnginesBox {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }
        self_.details.set_window(window);
        self_.window.set(window.clone()).unwrap();

        let factory = gtk4::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let row = EpicEngine::new();
            item.set_child(Some(&row));
        });

        factory.connect_bind(move |_factory, list_item| {
            let data = list_item
                .item()
                .unwrap()
                .downcast::<crate::models::engine_data::EngineData>()
                .unwrap();

            let child = list_item.child().unwrap().downcast::<EpicEngine>().unwrap();
            child.set_data(&data);

            child.set_property("branch", &data.branch());
            child.set_property("has-branch", &data.has_branch());
            child.set_property("needs-update", &data.needs_update());
        });

        let sorter = gtk4::CustomSorter::new(move |obj1, obj2| {
            let info1 = obj1
                .downcast_ref::<crate::models::engine_data::EngineData>()
                .unwrap();
            let info2 = obj2
                .downcast_ref::<crate::models::engine_data::EngineData>()
                .unwrap();

            match version_compare::compare(
                &info1.version().unwrap_or_default(),
                &info2.version().unwrap_or_default(),
            ) {
                Ok(comp) => match comp {
                    Cmp::Lt => gtk4::Ordering::Larger,
                    Cmp::Eq | Cmp::Le | Cmp::Ge => gtk4::Ordering::Equal,
                    Cmp::Gt | Cmp::Ne => gtk4::Ordering::Smaller,
                },
                Err(_) => gtk4::Ordering::Equal,
            }
        });
        let sorted_model = gtk4::SortListModel::new(Some(&self_.grid_model), Some(&sorter));
        let selection_model = gtk4::SingleSelection::new(Some(&sorted_model));
        selection_model.set_autoselect(false);
        selection_model.set_can_unselect(true);
        self_.engine_grid.set_model(Some(&selection_model));
        self_.engine_grid.set_factory(Some(&factory));

        selection_model.connect_selected_notify(clone!(@weak self as engines => move |model| {
            engines.engine_selected(model);
        }));
        self.load_engines();
    }

    fn engine_selected(&self, model: &gtk4::SingleSelection) {
        let self_ = self.imp();
        if let Some(a) = model.selected_item() {
            let engine = a
                .downcast::<crate::models::engine_data::EngineData>()
                .unwrap();
            self.set_property("selected", engine.path());
            self_.details.set_property("expanded", true);
            self_.details.set_data(&engine);
        }
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        self.insert_action_group("engines", Some(&self_.actions));
        action!(
            self_.actions,
            "add",
            clone!(@weak self as engines => move |_, _| {
                let self_ = engines.imp();
                self_.details.set_property("expanded", true);
                self_.details.add_engine();
            })
        );
    }

    pub fn selected(&self) -> Option<String> {
        self.property("selected")
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
        self_.details.set_download_manager(dm);
    }

    pub fn load_engines(&self) {
        let self_ = self.imp();
        'outer: for (guid, path) in Self::read_engines_ini() {
            let mut engines = self_.engines.borrow_mut();
            if let Some(version) =
                crate::models::engine_data::EngineData::read_engine_version(&path)
            {
                for eng in engines.values() {
                    if eng.path.eq(&path) {
                        engines.insert(
                            guid.clone(),
                            UnrealEngine {
                                version: version.clone(),
                                path: path.clone(),
                                guid: Some(guid.clone()),
                            },
                        );
                        continue 'outer;
                    }
                }
                engines.insert(
                    guid.clone(),
                    UnrealEngine {
                        version: version.clone(),
                        path: path.clone(),
                        guid: Some(guid.clone()),
                    },
                );
                let data = crate::models::engine_data::EngineData::new(
                    &path,
                    &guid,
                    &version,
                    &self_.grid_model,
                );

                self_.grid_model.append(&data);
            };
        }
        self.load_engines_from_fs();
    }

    pub fn load_engines_from_fs(&self) {
        let self_ = self.imp();
        'outer: for dir in self_.settings.strv("unreal-engine-directories") {
            match crate::models::engine_data::EngineData::read_engine_version(&dir.to_string()) {
                None => {
                    let path = std::path::PathBuf::from(dir.to_string());
                    if let Ok(rd) = path.read_dir() {
                        'inner: for d in rd.flatten() {
                            let p = d.path();
                            if p.is_dir() {
                                if let Some(version) =
                                    crate::models::engine_data::EngineData::read_engine_version(
                                        p.to_str().unwrap(),
                                    )
                                {
                                    let mut engines = self_.engines.borrow_mut();
                                    for engine in engines.values() {
                                        if engine.path.eq(p.to_str().unwrap()) {
                                            continue 'inner;
                                        }
                                    }
                                    engines.insert(
                                        p.to_str().unwrap().to_string(),
                                        UnrealEngine {
                                            version: version.clone(),
                                            path: p.to_str().unwrap().to_string(),
                                            guid: Some(p.to_str().unwrap().to_string()),
                                        },
                                    );
                                    let data = crate::models::engine_data::EngineData::new(
                                        p.to_str().unwrap(),
                                        p.to_str().unwrap(),
                                        &version,
                                        &self_.grid_model,
                                    );

                                    self_.grid_model.append(&data);
                                }
                            }
                        }
                    }
                }
                Some(version) => {
                    let mut engines = self_.engines.borrow_mut();
                    for engine in engines.values() {
                        if engine.path.eq(&dir.to_string()) {
                            continue 'outer;
                        }
                    }
                    engines.insert(
                        dir.to_string(),
                        UnrealEngine {
                            version: version.clone(),
                            path: dir.to_string(),
                            guid: Some(dir.to_string()),
                        },
                    );
                    let data = crate::models::engine_data::EngineData::new(
                        &dir,
                        &dir,
                        &version,
                        &self_.grid_model,
                    );

                    self_.grid_model.append(&data);
                }
            }
        }
    }

    pub fn read_engines_ini() -> HashMap<String, String> {
        let ini = gtk4::glib::KeyFile::new();
        let mut dir = gtk4::glib::home_dir();
        // TODO: This is not platform independent, Linux only
        dir.push(".config");
        dir.push("Epic");
        dir.push("UnrealEngine");
        dir.push("Install.ini");
        let mut result: HashMap<String, String> = HashMap::new();
        if let Err(e) = ini.load_from_file(dir, gtk4::glib::KeyFileFlags::NONE) {
            warn!("Unable to load engine Install.ini: {}", e);
            return result;
        };

        if let Ok(keys) = ini.keys("Installations") {
            for item in keys.0 {
                if let Ok(path) = ini.value("Installations", &item) {
                    let guid: String = item.chars().filter(|c| c != &'{' && c != &'}').collect();
                    debug!("Got engine install: {} in {}", guid, path);
                    match path.to_string().strip_suffix('/') {
                        None => {
                            result.insert(guid.to_string(), path.to_string());
                        }
                        Some(pa) => {
                            result.insert(guid.to_string(), pa.to_string());
                        }
                    }
                }
            }
        }
        result
    }

    pub fn update_docker(&self) {
        let self_ = self.imp();
        self_.details.update_docker();
    }

    pub fn engine_from_assoociation(&self, engine_association: &str) -> Option<UnrealEngine> {
        let self_ = self.imp();
        if let Some(engine) = self_.engines.borrow().get(engine_association) {
            return Some(engine.clone());
        };
        for engine in self_.engines.borrow().values() {
            if engine_association.eq(&format!(
                "{}.{}",
                engine.version.major_version, engine.version.minor_version
            )) {
                return Some(engine.clone());
            }
        }
        None
    }

    pub fn engines(&self) -> Vec<UnrealEngine> {
        let self_ = self.imp();
        let mut result: Vec<UnrealEngine> = Vec::new();
        for engine in self_.engines.borrow().values() {
            result.push(engine.clone());
        }
        result.sort_by(|a, b| a.version.compare(&b.version));
        result
    }
}
