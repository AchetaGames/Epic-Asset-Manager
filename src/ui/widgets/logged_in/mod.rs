mod asset;
pub mod category;

use crate::tools::asset_info::Search;
use crate::ui::widgets::logged_in::asset::EpicAsset;
use glib::clone;
use gtk::{self, gdk_pixbuf, prelude::*};
use gtk::{gio, glib, subclass::prelude::*, CompositeTemplate};
use gtk_macros::action;
use log::debug;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::ops::Not;
use std::path::{Path, PathBuf};

pub(crate) mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk::gio;
    use gtk::gio::ListStore;
    use gtk::glib::{Object, ParamSpec};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use threadpool::ThreadPool;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/logged_in.ui")]
    pub struct EpicLoggedInBox {
        #[template_child]
        pub home_category:
            TemplateChild<crate::ui::widgets::logged_in::category::EpicSidebarCategory>,
        #[template_child]
        pub assets_category:
            TemplateChild<crate::ui::widgets::logged_in::category::EpicSidebarCategory>,
        #[template_child]
        pub plugins_category:
            TemplateChild<crate::ui::widgets::logged_in::category::EpicSidebarCategory>,
        #[template_child]
        pub games_category:
            TemplateChild<crate::ui::widgets::logged_in::category::EpicSidebarCategory>,
        #[template_child]
        pub other_category:
            TemplateChild<crate::ui::widgets::logged_in::category::EpicSidebarCategory>,
        #[template_child]
        pub projects_category:
            TemplateChild<crate::ui::widgets::logged_in::category::EpicSidebarCategory>,
        #[template_child]
        pub expand_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub expand_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub expand_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub asset_grid: TemplateChild<gtk::GridView>,
        #[template_child]
        pub asset_search: TemplateChild<gtk::SearchEntry>,
        pub sidebar_expanded: RefCell<bool>,
        pub filter: RefCell<Option<String>>,
        pub search: RefCell<Option<String>>,
        pub actions: gio::SimpleActionGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub filter_model: gtk::FilterListModel,
        pub grid_model: ListStore,
        pub loaded_assets: RefCell<HashMap<String, egs_api::api::types::asset_info::AssetInfo>>,
        pub asset_load_pool: ThreadPool,
        pub image_load_pool: ThreadPool,
        pub assets_pending: Arc<std::sync::RwLock<Vec<Object>>>,
        pub categories: RefCell<HashSet<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicLoggedInBox {
        const NAME: &'static str = "EpicLoggedInBox";
        type Type = super::EpicLoggedInBox;
        type ParentType = gtk::Box;

        fn new() -> Self {
            Self {
                home_category: TemplateChild::default(),
                assets_category: TemplateChild::default(),
                plugins_category: TemplateChild::default(),
                games_category: TemplateChild::default(),
                other_category: TemplateChild::default(),
                projects_category: TemplateChild::default(),
                expand_button: TemplateChild::default(),
                expand_image: TemplateChild::default(),
                expand_label: TemplateChild::default(),
                asset_grid: TemplateChild::default(),
                asset_search: TemplateChild::default(),
                sidebar_expanded: RefCell::new(false),
                filter: RefCell::new(None),
                search: RefCell::new(None),
                actions: gio::SimpleActionGroup::new(),
                window: OnceCell::new(),
                filter_model: gtk::FilterListModel::new(gio::NONE_LIST_MODEL, gtk::NONE_FILTER),
                grid_model: gio::ListStore::new(crate::models::row_data::RowData::static_type()),
                loaded_assets: RefCell::new(HashMap::new()),
                asset_load_pool: ThreadPool::with_name("Asset Load Pool".to_string(), 5),
                image_load_pool: ThreadPool::with_name("Image Load Pool".to_string(), 5),
                assets_pending: Arc::new(std::sync::RwLock::new(vec![])),
                categories: RefCell::new(HashSet::new()),
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

    impl ObjectImpl for EpicLoggedInBox {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_boolean(
                        "sidebar-expanded",
                        "sidebar expanded",
                        "Is Sidebar expanded",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "filter",
                        "Filter",
                        "Filter",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "search",
                        "Search",
                        "Search",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "sidebar-expanded" => {
                    let sidebar_expanded = value.get().unwrap();
                    self.sidebar_expanded.replace(sidebar_expanded);
                }
                "filter" => {
                    let filter: Option<String> = value.get().unwrap();

                    self.filter.replace(match filter {
                        None => None,
                        Some(f) => {
                            if f.is_empty() {
                                None
                            } else {
                                Some(f)
                            }
                        }
                    });
                    obj.unselect_categories_except();
                    obj.apply_filter();
                }
                "search" => {
                    let search: Option<String> = value.get().unwrap();
                    self.search.replace(match search {
                        None => None,
                        Some(f) => {
                            if f.is_empty() {
                                None
                            } else {
                                Some(f)
                            }
                        }
                    });
                    obj.apply_filter();
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "sidebar-expanded" => self.sidebar_expanded.borrow().to_value(),
                "filter" => self.filter.borrow().to_value(),
                "search" => self.search.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.bind_properties();
            obj.setup_actions();
            obj.setup_widgets();
            self.home_category
                .add_category("all".to_string(), "".to_string());
            self.home_category
                .add_category("favorites".to_string(), "favorites".to_string());
        }
    }

    impl WidgetImpl for EpicLoggedInBox {}
    impl BoxImpl for EpicLoggedInBox {}
}

glib::wrapper! {
    pub struct EpicLoggedInBox(ObjectSubclass<imp::EpicLoggedInBox>)
        @extends gtk::Widget, gtk::Box;
}

impl EpicLoggedInBox {
    pub fn new() -> Self {
        let stack: Self = glib::Object::new(&[]).expect("Failed to create EpicLoggedInBox");

        stack
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        // Do not run this twice
        if let Some(_) = self_.window.get() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let row = EpicAsset::new();
            item.set_child(Some(&row));
        });

        factory.connect_bind(move |_factory, list_item| {
            let data = list_item
                .item()
                .unwrap()
                .downcast::<crate::models::row_data::RowData>()
                .unwrap();

            let child = list_item.child().unwrap().downcast::<EpicAsset>().unwrap();
            child.set_property("label", &data.name()).unwrap();
            child.set_property("thumbnail", &data.image()).unwrap();
        });

        let sorter = gtk::CustomSorter::new(move |obj1, obj2| {
            let info1 = obj1
                .downcast_ref::<crate::models::row_data::RowData>()
                .unwrap();
            let info2 = obj2
                .downcast_ref::<crate::models::row_data::RowData>()
                .unwrap();

            info1
                .name()
                .to_lowercase()
                .cmp(&info2.name().to_lowercase())
                .into()
        });

        self_.filter_model.set_model(Some(&self_.grid_model));
        let sorted_model = gtk::SortListModel::new(Some(&self_.filter_model), Some(&sorter));
        let selection_model = gtk::SingleSelection::new(Some(&sorted_model));
        selection_model.set_autoselect(false);
        selection_model.set_can_unselect(true);
        self_.asset_grid.set_model(Some(&selection_model));
        self_.asset_grid.set_factory(Some(&factory));

        selection_model.connect_selected_notify(clone!(@weak self as loggedin => move |model| {
            if let Some(a) = model.selected_item() {
                let asset = a.downcast::<crate::models::row_data::RowData>().unwrap();
                println!("Selected: {}", asset.name());
            }
        }));

        self.fetch_assets();
    }

    pub fn flush_assets(&self) {
        let start = std::time::Instant::now();
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        if let Ok(mut vec) = self_.assets_pending.write() {
            if vec.is_empty() {
                return;
            }
            self_.grid_model.splice(0, 0, vec.as_slice());
            vec.clear();
        }
        debug!("Finished flushing {:?}", start.elapsed());
    }

    pub fn bind_properties(&self) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        self.bind_property("sidebar-expanded", &*self_.home_category, "expanded")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        self.bind_property("sidebar-expanded", &*self_.assets_category, "expanded")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        self.bind_property("sidebar-expanded", &*self_.plugins_category, "expanded")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        self.bind_property("sidebar-expanded", &*self_.games_category, "expanded")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        self.bind_property("sidebar-expanded", &*self_.other_category, "expanded")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        self.bind_property("sidebar-expanded", &*self_.projects_category, "expanded")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        self_
            .asset_search
            .bind_property("text", self, "search")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
    }

    pub fn setup_widgets(&self) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        self_.projects_category.set_logged_in(self);
        self_.assets_category.set_logged_in(self);
        self_.plugins_category.set_logged_in(self);
        self_.other_category.set_logged_in(self);
        self_.games_category.set_logged_in(self);
        self_.home_category.set_logged_in(self);
    }

    pub fn setup_actions(&self) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);

        action!(
            self_.actions,
            "expand",
            clone!(@weak self as win => move |_, _| {
                    if let Ok(v) = win.property("sidebar-expanded") {
                    let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(&win);
                    let new_value = !v.get::<bool>().unwrap();
                    if new_value {
                        self_.expand_image.set_icon_name(Some("go-previous-symbolic"));
                        self_.expand_button.set_tooltip_text(Some("Collapse Sidebar"));
                        self_.expand_label.set_label("Collapse");
                    } else {
                        self_.expand_image.set_icon_name(Some("go-next-symbolic"));
                        self_.expand_button.set_tooltip_text(Some("Expand Sidebar"));
                        self_.expand_label.set_label("");
                    };
                    win.set_property("sidebar-expanded", &new_value).unwrap();
                }
            })
        );
        self.insert_action_group("loggedin", Some(&self_.actions));
    }

    pub fn filter(&self) -> Option<String> {
        if let Ok(value) = self.property("filter") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        return None;
    }

    pub fn search(&self) -> Option<String> {
        if let Ok(value) = self.property("search") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        return None;
    }

    pub fn unselect_categories_except(&self) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        let filter = match self.filter() {
            None => "".to_string(),
            Some(f) => f,
        };
        self_.projects_category.unselect_except(&filter);
        self_.assets_category.unselect_except(&filter);
        self_.plugins_category.unselect_except(&filter);
        self_.other_category.unselect_except(&filter);
        self_.games_category.unselect_except(&filter);
        self_.home_category.unselect_except(&filter);
    }

    pub fn apply_filter(&self) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        let search = self.search();
        let filter = self.filter();
        println!("Search {:?} ", search);
        println!("Filter {:?} ", filter);

        let filter = gtk::CustomFilter::new(move |object| {
            let asset = object
                .downcast_ref::<crate::models::row_data::RowData>()
                .unwrap();
            (match &search {
                None => true,
                Some(se) => asset
                    .name()
                    .to_ascii_lowercase()
                    .contains(&se.to_ascii_lowercase()),
            }) && (match &filter {
                None => true,
                Some(f) => asset.check_category(f.clone()),
            })
        });
        self_.filter_model.set_filter(Some(&filter));
    }

    pub fn add_asset(&self, asset: egs_api::api::types::asset_info::AssetInfo, image: &[u8]) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        if let Some(categories) = &asset.categories {
            for category in categories {
                let mut cats = self_.categories.borrow_mut();
                if cats.insert(category.path.clone()) {
                    self.add_category(&category.path);
                }
            }
        };
        let mut assets = self_.loaded_assets.borrow_mut();
        if match assets.get_mut(&asset.id) {
            None => {
                assets.insert(asset.id.clone(), asset.clone());
                true
            }
            Some(a) => {
                if asset.eq(a) {
                    debug!("Duplicate asset: {}", asset.id);
                    false
                } else {
                    assets.insert(asset.id.clone(), asset.clone());
                    true
                }
            }
        } {
            let data = crate::models::row_data::RowData::new(asset, image);
            if let Ok(mut vec) = self_.assets_pending.write() {
                vec.push(data.upcast());
            }
        }
    }

    fn add_category(&self, path: &str) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        let parts = path.split("/").collect::<Vec<&str>>();
        if parts.len() > 1 {
            let name = parts[1..].join("/");
            match parts[0] {
                "assets" => &self_.assets_category,
                "plugins" => &self_.plugins_category,
                "projects" => &self_.projects_category,
                &_ => &self_.other_category,
            }
            .add_category(name, path.to_string());
        }
    }

    pub fn load_thumbnail(&self, asset: egs_api::api::types::asset_info::AssetInfo) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        if let Some(window) = self.main_window() {
            let win_ = window.data();
            let sender = win_.model.sender.clone();
            match asset.thumbnail() {
                None => {
                    sender
                        .send(crate::ui::messages::Msg::ProcessAssetThumbnail(
                            asset,
                            vec![],
                        ))
                        .unwrap();
                }
                Some(t) => {
                    let cache_dir = win_
                        .model
                        .settings
                        .string("cache-directory")
                        .to_string()
                        .clone();
                    let mut cache_path = PathBuf::from(cache_dir);
                    cache_path.push("images");
                    let name = Path::new(t.url.path()).extension().and_then(OsStr::to_str);
                    cache_path.push(format!("{}.{}", t.md5, name.unwrap_or(&".png")));
                    self_.image_load_pool.execute(move || {
                        match File::open(cache_path.as_path()) {
                            Ok(mut f) => {
                                let metadata = fs::metadata(&cache_path.as_path())
                                    .expect("unable to read metadata");
                                let mut buffer = vec![0; metadata.len() as usize];
                                f.read(&mut buffer).expect("buffer overflow");
                                let pixbuf_loader = gdk_pixbuf::PixbufLoader::new();
                                pixbuf_loader.write(&buffer).unwrap();
                                pixbuf_loader.close().ok();
                                match pixbuf_loader.pixbuf() {
                                    None => {}
                                    Some(pb) => {
                                        let width = pb.width();
                                        let height = pb.height();

                                        let width_percent = 128.0 / width as f64;
                                        let height_percent = 128.0 / height as f64;
                                        let percent = if height_percent < width_percent {
                                            height_percent
                                        } else {
                                            width_percent
                                        };
                                        let desired =
                                            (width as f64 * percent, height as f64 * percent);
                                        sender
                                            .send(crate::ui::messages::Msg::ProcessAssetThumbnail(
                                                asset,
                                                pb.scale_simple(
                                                    desired.0.round() as i32,
                                                    desired.1.round() as i32,
                                                    gdk_pixbuf::InterpType::Bilinear,
                                                )
                                                .unwrap()
                                                .save_to_bufferv("png", &[])
                                                .unwrap(),
                                            ))
                                            .unwrap()
                                    }
                                };
                            }
                            Err(_) => {
                                println!("Need to load image");
                            }
                        };
                    })
                }
            }
        }
    }

    fn main_window(&self) -> Option<&crate::window::EpicAssetManagerWindow> {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        match self_.window.get() {
            Some(window) => Some(&(*window)),
            None => None,
        }
    }

    pub fn fetch_assets(&self) {
        let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(self);
        if let Some(window) = self.main_window() {
            let win_ = window.data();
            let sender = win_.model.sender.clone();
            let cache_dir = win_
                .model
                .settings
                .string("cache-directory")
                .to_string()
                .clone();
            self_.asset_load_pool.execute(move || {
                // Load assets from cache
                let cache_path = PathBuf::from(cache_dir);
                if cache_path.is_dir() {
                    for entry in std::fs::read_dir(cache_path).unwrap() {
                        if let Ok(w) = crate::RUNNING.read() {
                            if w.not() {
                                break;
                            }
                        }
                        let mut asset_file = entry.unwrap().path();
                        asset_file.push("asset_info.json");
                        if asset_file.exists() {
                            if let Ok(mut f) = std::fs::File::open(asset_file.as_path()) {
                                let mut buffer = String::new();
                                f.read_to_string(&mut buffer).unwrap();
                                if let Ok(asset) = serde_json::from_str::<
                                    egs_api::api::types::asset_info::AssetInfo,
                                >(&buffer)
                                {
                                    sender
                                        .send(crate::ui::messages::Msg::ProcessAssetInfo(asset))
                                        .unwrap();
                                }
                            };
                        }
                    }
                }
                // TODO: Update from the API
            });
            glib::idle_add_local(clone!(@weak self as obj => @default-panic, move || {
                obj.flush_assets();
                let self_: &imp::EpicLoggedInBox = imp::EpicLoggedInBox::from_instance(&obj);
                glib::Continue((self_.asset_load_pool.queued_count() +
                    self_.asset_load_pool.active_count() +
                    self_.image_load_pool.queued_count() +
                    self_.image_load_pool.active_count()) > 0)
            }));
        }
    }
}
