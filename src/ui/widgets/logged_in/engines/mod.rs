use crate::ui::widgets::logged_in::refresh::Refresh;
use adw::gtk;
use engine::EpicEngine;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use log::{debug, error, warn};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::str::FromStr;
use version_compare::Cmp;

mod docker_download;
pub mod engine;
pub mod engine_detail;
mod engines_side;
pub mod epic_download;
mod install;

pub enum Msg {
    AddEngine {
        guid: String,
        path: String,
        version: crate::models::engine_data::UnrealVersion,
    },
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
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

pub mod imp {
    use std::cell::RefCell;

    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString};
    use once_cell::sync::OnceCell;
    use threadpool::ThreadPool;

    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/engines.ui")]
    pub struct EpicEnginesBox {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        #[template_child]
        pub engine_grid: TemplateChild<gtk4::GridView>,
        #[template_child]
        pub side: TemplateChild<engines_side::EpicEnginesSide>,
        pub grid_model: gio::ListStore,
        pub expanded: RefCell<bool>,
        pub file_pool: ThreadPool,
        selected: RefCell<Option<String>>,
        pub actions: gio::SimpleActionGroup,
        pub engines: RefCell<HashMap<String, UnrealEngine>>,
        pub settings: gio::Settings,
        pub sender: async_channel::Sender<Msg>,
        pub receiver: RefCell<Option<async_channel::Receiver<Msg>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEnginesBox {
        const NAME: &'static str = "EpicEnginesBox";
        type Type = super::EpicEnginesBox;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            let (sender, receiver) = async_channel::unbounded();
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                engine_grid: TemplateChild::default(),
                side: TemplateChild::default(),
                grid_model: gtk4::gio::ListStore::new::<crate::models::engine_data::EngineData>(),
                expanded: RefCell::new(false),
                selected: RefCell::new(None),
                actions: gtk4::gio::SimpleActionGroup::new(),
                engines: RefCell::new(HashMap::new()),
                settings: gio::Settings::new(crate::config::APP_ID),
                sender,
                receiver: RefCell::new(Some(receiver)),
                file_pool: ThreadPool::with_name("File Pool".to_string(), 1),
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
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            obj.setup_messaging();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("expanded").build(),
                    ParamSpecString::builder("selected").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
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

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
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
        glib::Object::new()
    }

    pub fn setup_messaging(&self) {
        let self_ = self.imp();
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        glib::spawn_future_local(clone!(
            #[weak(rename_to=engines)]
            self,
            #[upgrade_or_panic]
            async move {
                while let Ok(response) = receiver.recv().await {
                    engines.update(response);
                }
            }
        ));
    }

    fn update(&self, msg: Msg) {
        match msg {
            Msg::AddEngine {
                guid,
                path,
                version,
            } => self.add_engine(guid, path, version),
        }
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }
        self_.side.set_window(window);
        self_.window.set(window.clone()).unwrap();

        let factory = gtk4::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let row = EpicEngine::new();
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            item.set_child(Some(&row));
        });

        factory.connect_bind(move |_factory, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let data = list_item
                .item()
                .unwrap()
                .downcast::<crate::models::engine_data::EngineData>()
                .unwrap();

            let child = list_item.child().unwrap().downcast::<EpicEngine>().unwrap();
            child.set_data(&data);

            child.set_property("branch", data.branch());
            child.set_property("has-branch", data.has_branch());
            child.set_property("needs-update", data.needs_update());
        });

        let sorter = gtk4::CustomSorter::new(move |obj1, obj2| {
            let info1 = obj1
                .downcast_ref::<crate::models::engine_data::EngineData>()
                .unwrap();
            let info2 = obj2
                .downcast_ref::<crate::models::engine_data::EngineData>()
                .unwrap();

            if !info1.valid() {
                return gtk4::Ordering::Larger;
            }
            if !info2.valid() {
                return gtk4::Ordering::Smaller;
            }

            version_compare::compare(
                info1.version().unwrap_or_default(),
                info2.version().unwrap_or_default(),
            )
            .map_or(gtk4::Ordering::Smaller, |comp| match comp {
                Cmp::Lt => gtk4::Ordering::Larger,
                Cmp::Eq | Cmp::Le | Cmp::Ge => gtk4::Ordering::Equal,
                Cmp::Gt | Cmp::Ne => gtk4::Ordering::Smaller,
            })
        });
        let sorted_model = gtk4::SortListModel::builder()
            .model(&self_.grid_model)
            .sorter(&sorter)
            .build();
        let selection_model = gtk4::SingleSelection::builder()
            .model(&sorted_model)
            .autoselect(false)
            .can_unselect(true)
            .build();
        self_.engine_grid.set_model(Some(&selection_model));
        self_.engine_grid.set_factory(Some(&factory));

        selection_model.connect_selected_notify(clone!(
            #[weak(rename_to=engines)]
            self,
            move |model| {
                engines.engine_selected(model);
            }
        ));

        let data = crate::models::engine_data::EngineData::new(
            "",
            "",
            &crate::models::engine_data::UnrealVersion {
                major_version: -1,
                minor_version: -1,
                patch_version: -1,
                changelist: -1,
                compatible_changelist: -1,
                is_licensee_version: -1,
                is_promoted_build: -1,
                branch_name: "Install Engine".to_string(),
            },
            &self_.grid_model,
        );

        self_.grid_model.append(&data);

        self.load_engines();
        glib::timeout_add_seconds_local(
            15 * 60 + (rand::random::<u32>() % 5) * 60,
            clone!(
                #[weak(rename_to=obj)]
                self,
                #[upgrade_or_panic]
                move || {
                    obj.run_refresh();
                    glib::ControlFlow::Continue
                }
            ),
        );
    }

    fn engine_selected(&self, model: &gtk4::SingleSelection) {
        let self_ = self.imp();
        if let Some(a) = model.selected_item() {
            let engine = a
                .downcast::<crate::models::engine_data::EngineData>()
                .unwrap();
            self_.side.set_property("position", model.selected());
            self_.side.set_property("expanded", true);
            if engine.valid() {
                self.set_property("selected", engine.path());
                self_.side.set_data(&engine);
            } else {
                self_.side.add_engine();
            }
        }
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        self.insert_action_group("engines", Some(&self_.actions));
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
        self_.side.set_download_manager(dm);
    }

    fn add_engine(
        &self,
        guid: String,
        path: String,
        version: crate::models::engine_data::UnrealVersion,
    ) {
        let self_ = self.imp();
        let mut engines = self_.engines.borrow_mut();
        for eng in engines.values() {
            if eng.path.eq(&path) {
                engines.insert(
                    guid.clone(),
                    UnrealEngine {
                        version,
                        path,
                        guid: Some(guid),
                    },
                );
                return;
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

        let data =
            crate::models::engine_data::EngineData::new(&path, &guid, &version, &self_.grid_model);

        self_.grid_model.append(&data);
        self.refresh_state_changed();
    }

    pub fn remove_invalid(&self) {
        let self_ = self.imp();
        for item in self_.grid_model.snapshot() {
            let data = item
                .clone()
                .downcast::<crate::models::engine_data::EngineData>()
                .unwrap();
            // Skipping the Add Engine item
            if !data.valid() {
                continue;
            }
            data.path().map_or_else(
                || self.remove_item(&item, data.guid()),
                |path| {
                    PathBuf::from_str(&path).map_or_else(
                        |_| self.remove_item(&item, data.guid()),
                        |mut p| {
                            if !p.exists() {
                                self.remove_item(&item, data.guid());
                            }
                            p.push("Engine");
                            p.push("Build");
                            p.push("Build.version");
                            if !p.exists() {
                                self.remove_item(&item, data.guid());
                            }
                        },
                    );
                },
            );
        }
    }

    fn remove_item(&self, item: &gtk4::glib::Object, guid: Option<String>) {
        let self_ = self.imp();
        let engine = item
            .clone()
            .downcast::<crate::models::engine_data::EngineData>()
            .unwrap();
        if let Some(id) = self_.grid_model.find(item) {
            self_.grid_model.remove(id);
        }
        if let Some(g) = guid {
            self_.engines.borrow_mut().remove(&g);
        }
        if let Some(path) = engine.path() {
            if let Some(p) = self_.side.path() {
                if path.eq(&p) {
                    self_.side.collapse();
                }
            }
        }
    }

    pub fn load_engines(&self) {
        let self_ = self.imp();
        let s = self_.sender.clone();
        self.remove_invalid();
        self_.file_pool.execute(move || {
            for (guid, path) in Self::read_engines_ini() {
                if let Some(version) =
                    crate::models::engine_data::EngineData::read_engine_version(&path)
                {
                    s.send_blocking(Msg::AddEngine {
                        guid,
                        path,
                        version,
                    })
                    .unwrap();
                };
            }
        });
        self.load_engines_from_fs();
    }

    pub fn load_engines_from_fs(&self) {
        let self_ = self.imp();
        for dir in self_.settings.strv("unreal-engine-directories") {
            let s = self_.sender.clone();
            self_.file_pool.execute(move || {
                match crate::models::engine_data::EngineData::read_engine_version(&dir) {
                    None => {
                        let path = std::path::PathBuf::from(dir.to_string());
                        if let Ok(rd) = path.read_dir() {
                            for d in rd.flatten() {
                                if let Ok(w) = crate::RUNNING.read() {
                                    if !*w {
                                        return;
                                    }
                                }
                                let p = d.path();
                                if p.is_dir() {
                                    if let Some(version) =
                                        crate::models::engine_data::EngineData::read_engine_version(
                                            p.to_str().unwrap(),
                                        )
                                    {
                                        s.send_blocking(Msg::AddEngine {
                                            guid: p.to_str().unwrap().to_string(),
                                            path: p.to_str().unwrap().to_string(),
                                            version,
                                        })
                                        .unwrap();
                                    }
                                }
                            }
                        }
                    }
                    Some(version) => {
                        s.send_blocking(Msg::AddEngine {
                            guid: dir.to_string(),
                            path: dir.to_string(),
                            version,
                        })
                        .unwrap();
                    }
                }
            });
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
            for item in keys {
                if let Ok(path) = ini.value("Installations", item.as_str()) {
                    let guid: String = item
                        .to_string()
                        .chars()
                        .filter(|c| c != &'{' && c != &'}')
                        .collect();
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
        self_.side.update_docker();
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

impl Refresh for EpicEnginesBox {
    fn run_refresh(&self) {
        self.load_engines();
    }

    fn can_be_refreshed(&self) -> bool {
        let self_ = self.imp();
        self_.file_pool.queued_count() + self_.file_pool.active_count() == 0
    }

    fn refresh_state_changed(&self) {
        let self_ = self.imp();
        if let Some(w) = self_.window.get() {
            let w_ = w.imp();
            w_.logged_in_stack.tab_switched();
        }
    }
}
