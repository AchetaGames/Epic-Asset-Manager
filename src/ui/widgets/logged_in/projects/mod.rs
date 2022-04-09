use gtk4::{self, glib, glib::clone, prelude::*, subclass::prelude::*, CompositeTemplate};
use log::info;
use project::EpicProject;
use std::path::{Path, PathBuf};
mod project;
mod project_detail;

pub(crate) mod imp {
    use super::*;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicProjectsBox {
        const NAME: &'static str = "EpicProjectsBox";
        type Type = super::EpicProjectsBox;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                settings: gtk4::gio::Settings::new(crate::config::APP_ID),
                projects_grid: TemplateChild::default(),
                details: TemplateChild::default(),
                projects: RefCell::new(BTreeMap::new()),
                grid_model: gtk4::gio::ListStore::new(
                    crate::models::project_data::ProjectData::static_type(),
                ),
                expanded: RefCell::new(false),
                selected: RefCell::new(None),
                selected_uproject: RefCell::new(None),
                actions: gtk4::gio::SimpleActionGroup::new(),
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
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
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
            item.set_child(Some(&row));
        });

        factory.connect_bind(move |_factory, list_item| {
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
        let sorted_model = gtk4::SortListModel::new(Some(&self_.grid_model), Some(&sorter));
        let selection_model = gtk4::SingleSelection::new(Some(&sorted_model));
        selection_model.set_autoselect(false);
        selection_model.set_can_unselect(true);

        selection_model.connect_selected_notify(clone!(@weak self as projects => move |model| {
            projects.project_selected(model);
        }));
        self_.projects_grid.set_model(Some(&selection_model));
        self_.projects_grid.set_factory(Some(&factory));
        self.load_projects();
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

    fn load_projects(&self) {
        let self_ = self.imp();
        for dir in self_.settings.strv("unreal-projects-directories") {
            info!("Checking directory {}", dir);
            let path = std::path::PathBuf::from(dir.to_string());
            self.check_path_for_uproject(&path);
        }
    }

    fn add_project(&self, uproject_file: &PathBuf) {
        let self_ = self.imp();
        if let Some(directory) = uproject_file.parent() {
            if let Some(oname) = uproject_file.file_stem() {
                match oname.to_str() {
                    None => {}
                    Some(name) => {
                        self_
                            .projects
                            .borrow_mut()
                            .insert(directory.to_str().unwrap().to_string(), name.to_string());
                        self_
                            .grid_model
                            .append(&crate::models::project_data::ProjectData::new(
                                uproject_file.to_str().unwrap(),
                                name,
                            ));
                    }
                }
            }
        }
    }

    // TODO this should probably be done in a thread in case we loop
    fn check_path_for_uproject(&self, path: &Path) {
        if let Ok(rd) = path.read_dir() {
            for d in rd {
                match d {
                    Ok(entry) => {
                        let p = entry.path();
                        if p.is_dir() {
                            if let Some(uproject_file) = EpicProjectsBox::uproject_path(&p) {
                                self.add_project(&uproject_file);
                            } else {
                                self.check_path_for_uproject(&p);
                            };
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
