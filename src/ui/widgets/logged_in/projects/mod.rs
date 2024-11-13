use crate::ui::widgets::logged_in::refresh::Refresh;
use adw::gtk;
use gtk4::{self, glib, glib::clone, prelude::*, subclass::prelude::*, CompositeTemplate};
use log::{debug, info};
use project::EpicProject;
use std::path::{Path, PathBuf};
use std::str::FromStr;

mod project;
mod project_detail;

pub enum Msg {
    AddProject { uproject_file: PathBuf },
}

pub mod imp {
    use super::*;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use threadpool::ThreadPool;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/projects.ui")]
    pub struct EpicProjectsBox {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        pub settings: gtk4::gio::Settings,
        #[template_child]
        pub projects_grid: TemplateChild<gtk4::GridView>,
        #[template_child]
        pub details: TemplateChild<
            crate::ui::widgets::logged_in::projects::project_detail::UnrealProjectDetails,
        >,
        pub projects: RefCell<BTreeMap<String, String>>,
        pub grid_model: gtk4::gio::ListStore,
        pub expanded: RefCell<bool>,
        selected: RefCell<Option<String>>,
        pub selected_uproject: RefCell<Option<crate::models::project_data::Uproject>>,
        pub actions: gtk4::gio::SimpleActionGroup,
        pub sender: async_channel::Sender<Msg>,
        pub receiver: RefCell<Option<async_channel::Receiver<Msg>>>,
        pub file_pool: ThreadPool,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicProjectsBox {
        const NAME: &'static str = "EpicProjectsBox";
        type Type = super::EpicProjectsBox;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            let (sender, receiver) = async_channel::unbounded();
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                settings: gtk4::gio::Settings::new(crate::config::APP_ID),
                projects_grid: TemplateChild::default(),
                details: TemplateChild::default(),
                projects: RefCell::new(BTreeMap::new()),
                grid_model: gtk4::gio::ListStore::new::<crate::models::project_data::ProjectData>(),
                expanded: RefCell::new(false),
                selected: RefCell::new(None),
                selected_uproject: RefCell::new(None),
                actions: gtk4::gio::SimpleActionGroup::new(),
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

    impl ObjectImpl for EpicProjectsBox {
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

    impl WidgetImpl for EpicProjectsBox {}
    impl BoxImpl for EpicProjectsBox {}
}

glib::wrapper! {
    pub struct EpicProjectsBox(ObjectSubclass<imp::EpicProjectsBox>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicProjectsBox {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicProjectsBox {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn setup_messaging(&self) {
        let self_ = self.imp();
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        glib::spawn_future_local(clone!(
            #[weak(rename_to=projects)]
            self,
            #[upgrade_or_panic]
            async move {
                while let Ok(msg) = receiver.recv().await {
                    projects.update(msg);
                }
            }
        ));
    }

    fn update(&self, msg: Msg) {
        match msg {
            Msg::AddProject { uproject_file } => self.add_project(&uproject_file),
        }
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        self_.details.set_window(&window.clone());

        let factory = gtk4::SignalListItemFactory::new();
        let w = window.clone();
        factory.connect_setup(move |_factory, item| {
            let row = EpicProject::new();
            row.set_window(&w);
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            item.set_child(Some(&row));
        });

        factory.connect_bind(move |_factory, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let data = list_item
                .item()
                .unwrap()
                .downcast::<crate::models::project_data::ProjectData>()
                .unwrap();

            let child = list_item
                .child()
                .unwrap()
                .downcast::<EpicProject>()
                .unwrap();
            child.set_data(&data);
        });

        let sorter = gtk4::CustomSorter::new(move |obj1, obj2| {
            let info1 = obj1
                .downcast_ref::<crate::models::project_data::ProjectData>()
                .unwrap();
            let info2 = obj2
                .downcast_ref::<crate::models::project_data::ProjectData>()
                .unwrap();
            match info1.name() {
                None => gtk4::Ordering::Larger,
                Some(a) => a
                    .to_lowercase()
                    .cmp(&match info2.name() {
                        None => return gtk4::Ordering::Smaller,
                        Some(b) => b.to_lowercase(),
                    })
                    .into(),
            }
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

        selection_model.connect_selected_notify(clone!(
            #[weak(rename_to=projects)]
            self,
            move |model| {
                projects.project_selected(model);
            }
        ));
        self_.projects_grid.set_model(Some(&selection_model));
        self_.projects_grid.set_factory(Some(&factory));
        self.load_projects();
        glib::timeout_add_seconds_local(
            15 * 60 + (rand::random::<u32>() % 5) * 60,
            clone!(
                #[weak(rename_to=obj)]
                self,
                #[upgrade_or_panic]
                move || {
                    debug!("Starting timed projects refresh");
                    obj.run_refresh();
                    glib::ControlFlow::Continue
                }
            ),
        );
    }

    fn project_selected(&self, model: &gtk4::SingleSelection) {
        if let Some(a) = model.selected_item() {
            let self_ = self.imp();
            let project = a
                .downcast::<crate::models::project_data::ProjectData>()
                .unwrap();
            if let Some(uproject) = project.uproject() {
                self_.details.set_project(&uproject, project.path());
            }
            self_.details.set_property("position", model.selected());
            self.set_property("selected", project.path());
            self_.selected_uproject.replace(project.uproject());
            self.set_property("expanded", true);
        }
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        self.insert_action_group("projects", Some(&self_.actions));
    }

    pub fn remove_invalid(&self) {
        let self_ = self.imp();
        for item in self_.grid_model.snapshot() {
            let data = item
                .clone()
                .downcast::<crate::models::project_data::ProjectData>()
                .unwrap();
            data.path().map_or_else(
                || self.remove_item(&item, data.path()),
                |path| {
                    PathBuf::from_str(&path).map_or_else(
                        |_| self.remove_item(&item, data.path()),
                        |p| {
                            if !p.exists() {
                                self.remove_item(&item, data.path());
                            }
                        },
                    );
                },
            );
        }
    }

    fn remove_item(&self, item: &gtk4::glib::Object, path: Option<String>) {
        let self_ = self.imp();
        if let Some(id) = self_.grid_model.find(item) {
            self_.grid_model.remove(id);
        }
        if let Some(p) = path {
            if let Some(project) = self_.details.path() {
                if project.eq(&p) {
                    self_.details.collapse();
                }
            }
            if let Ok(file) = PathBuf::from_str(&p) {
                if let Some(directory) = file.parent() {
                    self_
                        .projects
                        .borrow_mut()
                        .remove(directory.to_str().unwrap());
                }
            }
        }
    }

    fn load_projects(&self) {
        let self_ = self.imp();
        self.remove_invalid();
        for dir in self_.settings.strv("unreal-projects-directories") {
            info!("Checking directory {}", dir);
            let path = std::path::PathBuf::from(dir.to_string());
            let s = self_.sender.clone();
            self_.file_pool.execute(move || {
                Self::check_path_for_uproject(&path, &s);
            });
        }
    }

    fn add_project(&self, uproject_file: &Path) {
        let self_ = self.imp();
        if let Some(directory) = uproject_file.parent() {
            if let Some(oname) = uproject_file.file_stem() {
                if let Some(name) = oname.to_str() {
                    if self_
                        .projects
                        .borrow_mut()
                        .insert(directory.to_str().unwrap().to_string(), name.to_string())
                        .is_none()
                    {
                        self_
                            .grid_model
                            .append(&crate::models::project_data::ProjectData::new(
                                uproject_file.to_str().unwrap(),
                                name,
                            ));
                    };
                }
            }
        }
        self.refresh_state_changed();
    }

    fn check_path_for_uproject(path: &Path, sender: &async_channel::Sender<Msg>) {
        if let Ok(rd) = path.read_dir() {
            for d in rd {
                match d {
                    Ok(entry) => {
                        let p = entry.path();
                        if p.is_dir() {
                            EpicProjectsBox::uproject_path(&p).map_or_else(
                                || {
                                    Self::check_path_for_uproject(&p, &sender.clone());
                                },
                                |uproject_file| {
                                    sender
                                        .send_blocking(Msg::AddProject { uproject_file })
                                        .unwrap();
                                },
                            );
                        } else {
                            continue;
                        }
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
        }
    }

    fn uproject_path(p: &Path) -> Option<PathBuf> {
        if let Ok(r) = p.read_dir() {
            for file_entry in r.flatten() {
                let file = file_entry.path();
                if file.is_file() {
                    if let Some(ext) = file.extension() {
                        if ext.eq("uproject") {
                            return Some(file);
                        }
                    }
                }
            }
        };
        None
    }
}

impl crate::ui::widgets::logged_in::refresh::Refresh for EpicProjectsBox {
    fn run_refresh(&self) {
        self.load_projects();
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
