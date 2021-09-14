use crate::ui::widgets::logged_in::engine::EpicEngine;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};
use log::{debug, warn};
use std::collections::HashMap;
use version_compare::{CompOp, VersionCompare};

pub(crate) mod imp {
    use super::*;
    use once_cell::sync::OnceCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/engines.ui")]
    pub struct EpicEnginesBox {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        #[template_child]
        pub engine_grid: TemplateChild<gtk4::GridView>,
        pub grid_model: gtk4::gio::ListStore,
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
                grid_model: gtk4::gio::ListStore::new(
                    crate::models::engine_data::EngineData::static_type(),
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

    impl ObjectImpl for EpicEnginesBox {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
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
        let self_: &imp::EpicEnginesBox = imp::EpicEnginesBox::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

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
            child.set_property("path", &data.path()).unwrap();
            child.set_property("guid", &data.guid()).unwrap();
        });

        let sorter = gtk4::CustomSorter::new(move |obj1, obj2| {
            let info1 = obj1
                .downcast_ref::<crate::models::engine_data::EngineData>()
                .unwrap();
            let info2 = obj2
                .downcast_ref::<crate::models::engine_data::EngineData>()
                .unwrap();

            match VersionCompare::compare(&info1.version(), &info2.version()) {
                Ok(comp) => match comp {
                    CompOp::Eq => gtk4::Ordering::Equal,
                    CompOp::Lt => gtk4::Ordering::Larger,
                    CompOp::Le => gtk4::Ordering::Equal,
                    CompOp::Ge => gtk4::Ordering::Equal,
                    CompOp::Gt => gtk4::Ordering::Smaller,
                    CompOp::Ne => gtk4::Ordering::Smaller,
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
        self.load_engines();
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_: &imp::EpicEnginesBox = imp::EpicEnginesBox::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn load_engines(&self) {
        let self_: &imp::EpicEnginesBox = imp::EpicEnginesBox::from_instance(self);
        for (guid, path) in Self::read_engines_ini() {
            let version = EpicEngine::read_engine_version(&path);
            let data = crate::models::engine_data::EngineData::new(path, guid, version);
            self_.grid_model.append(&data);
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
                    debug!("Got engine install: {} in {}", item, path);
                    result.insert(item.to_string(), path.to_string());
                }
            }
        }
        result
    }
}
