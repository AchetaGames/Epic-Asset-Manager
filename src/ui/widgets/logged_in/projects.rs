use crate::ui::widgets::logged_in::project::EpicProject;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};
use log::info;
use std::path::PathBuf;

pub(crate) mod imp {
    use super::*;
    use once_cell::sync::OnceCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/projects.ui")]
    pub struct EpicProjectsBox {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        pub settings: gtk4::gio::Settings,
        #[template_child]
        pub projects_grid: TemplateChild<gtk4::GridView>,
        pub grid_model: gtk4::gio::ListStore,
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
                grid_model: gtk4::gio::ListStore::new(
                    crate::models::project_data::ProjectData::static_type(),
                ),
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
            obj.load_projects();
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
        let self_: &imp::EpicProjectsBox = imp::EpicProjectsBox::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();

        let factory = gtk4::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let row = EpicProject::new();
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
            child.set_property("name", &data.name()).unwrap();
        });

        let sorter = gtk4::CustomSorter::new(move |obj1, obj2| {
            let info1 = obj1
                .downcast_ref::<crate::models::project_data::ProjectData>()
                .unwrap();
            let info2 = obj2
                .downcast_ref::<crate::models::project_data::ProjectData>()
                .unwrap();
            info1
                .name()
                .to_lowercase()
                .cmp(&info2.name().to_lowercase())
                .into()
        });
        let sorted_model = gtk4::SortListModel::new(Some(&self_.grid_model), Some(&sorter));
        let selection_model = gtk4::SingleSelection::new(Some(&sorted_model));
        selection_model.set_autoselect(false);
        selection_model.set_can_unselect(true);
        self_.projects_grid.set_model(Some(&selection_model));
        self_.projects_grid.set_factory(Some(&factory));
    }

    pub fn load_projects(&self) {
        let self_: &imp::EpicProjectsBox = imp::EpicProjectsBox::from_instance(self);
        for dir in self_.settings.strv("unreal-projects-directories") {
            info!("Checking directory {}", dir);
            let path = std::path::PathBuf::from(dir.to_string());
            if let Ok(rd) = path.read_dir() {
                for d in rd {
                    match d {
                        Ok(entry) => {
                            let p = entry.path();
                            if p.is_dir() {
                                if let Some(uproject_file) = EpicProjectsBox::uproject_path(p) {
                                    self_.grid_model.append(
                                        &crate::models::project_data::ProjectData::new(
                                            uproject_file.to_str().unwrap().to_string(),
                                            uproject_file
                                                .file_stem()
                                                .unwrap()
                                                .to_str()
                                                .unwrap()
                                                .to_string(),
                                        ),
                                    )
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
    }

    fn uproject_path(p: PathBuf) -> Option<PathBuf> {
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

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_: &imp::EpicProjectsBox = imp::EpicProjectsBox::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }
}
