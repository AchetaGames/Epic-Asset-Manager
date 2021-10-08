use crate::tools::asset_info::Search;
use crate::ui::widgets::logged_in::asset::EpicAsset;
use glib::clone;
use gtk4::{self, gdk_pixbuf, prelude::*};
use gtk4::{gio, glib, subclass::prelude::*, CompositeTemplate};
use gtk_macros::action;
use log::debug;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) mod imp {
    use super::*;
    use crate::config;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::gio;
    use gtk4::gio::ListStore;
    use gtk4::glib::{Object, ParamSpec};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use threadpool::ThreadPool;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/library.ui")]
    pub struct EpicLibraryBox {
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
        pub details: TemplateChild<crate::ui::widgets::logged_in::asset_detail::EpicAssetDetails>,
        #[template_child]
        pub download_progress: TemplateChild<gtk4::ProgressBar>,
        #[template_child]
        pub expand_button: TemplateChild<gtk4::Button>,
        #[template_child]
        pub expand_image: TemplateChild<gtk4::Image>,
        #[template_child]
        pub expand_label: TemplateChild<gtk4::Label>,
        #[template_child]
        pub asset_grid: TemplateChild<gtk4::GridView>,
        #[template_child]
        pub asset_search: TemplateChild<gtk4::SearchEntry>,
        pub sidebar_expanded: RefCell<bool>,
        pub filter: RefCell<Option<String>>,
        pub search: RefCell<Option<String>>,
        pub actions: gio::SimpleActionGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub filter_model: gtk4::FilterListModel,
        pub grid_model: ListStore,
        pub loaded_assets: RefCell<HashMap<String, egs_api::api::types::asset_info::AssetInfo>>,
        pub asset_product_names: RefCell<HashMap<String, String>>,
        pub asset_load_pool: ThreadPool,
        pub image_load_pool: ThreadPool,
        pub assets_pending: Arc<std::sync::RwLock<Vec<Object>>>,
        pub categories: RefCell<HashSet<String>>,
        pub settings: gio::Settings,
        item: RefCell<Option<String>>,
        product: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicLibraryBox {
        const NAME: &'static str = "EpicLibraryBox";
        type Type = super::EpicLibraryBox;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                home_category: TemplateChild::default(),
                assets_category: TemplateChild::default(),
                plugins_category: TemplateChild::default(),
                games_category: TemplateChild::default(),
                other_category: TemplateChild::default(),
                projects_category: TemplateChild::default(),
                details: TemplateChild::default(),
                download_progress: TemplateChild::default(),
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
                download_manager: OnceCell::new(),
                filter_model: gtk4::FilterListModel::new(gio::NONE_LIST_MODEL, gtk4::NONE_FILTER),
                grid_model: gio::ListStore::new(crate::models::asset_data::AssetData::static_type()),
                loaded_assets: RefCell::new(HashMap::new()),
                asset_product_names: RefCell::new(HashMap::new()),
                asset_load_pool: ThreadPool::with_name("Asset Load Pool".to_string(), 5),
                image_load_pool: ThreadPool::with_name("Image Load Pool".to_string(), 5),
                assets_pending: Arc::new(std::sync::RwLock::new(vec![])),
                categories: RefCell::new(HashSet::new()),
                settings: gio::Settings::new(config::APP_ID),
                item: RefCell::new(None),
                product: RefCell::new(None),
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

    impl ObjectImpl for EpicLibraryBox {
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
                    ParamSpec::new_string(
                        "item",
                        "item",
                        "item",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "product",
                        "product",
                        "product",
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
                "item" => {
                    let item = value.get().unwrap();
                    self.product.replace(None);
                    self.item.replace(item);
                    obj.open_asset();
                }
                "product" => {
                    let product = value.get().unwrap();
                    self.item.replace(None);
                    self.product.replace(product);
                    obj.open_asset();
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "sidebar-expanded" => self.sidebar_expanded.borrow().to_value(),
                "filter" => self.filter.borrow().to_value(),
                "search" => self.search.borrow().to_value(),
                "item" => self.item.borrow().to_value(),
                "product" => self.product.borrow().to_value(),
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

    impl WidgetImpl for EpicLibraryBox {}
    impl BoxImpl for EpicLibraryBox {}
}

glib::wrapper! {
    pub struct EpicLibraryBox(ObjectSubclass<imp::EpicLibraryBox>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicLibraryBox {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicLibraryBox {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }

        dm.connect_local(
            "tick",
            false,
            clone!(@weak self as obj, @weak dm => @default-return None, move |_| {
                let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(&obj);
                self_.download_progress.set_fraction(dm.progress());
                None}),
        )
        .unwrap();
        self_.download_manager.set(dm.clone()).unwrap();
        self_.details.set_download_manager(dm);
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        self_.details.set_window(&window.clone());
        let factory = gtk4::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let row = EpicAsset::new();
            item.set_child(Some(&row));
        });

        factory.connect_bind(move |_factory, list_item| {
            let data = list_item
                .item()
                .unwrap()
                .downcast::<crate::models::asset_data::AssetData>()
                .unwrap();

            let child = list_item.child().unwrap().downcast::<EpicAsset>().unwrap();
            child.set_property("label", &data.name()).unwrap();
            child.set_property("thumbnail", &data.image()).unwrap();
            child.set_property("favorite", &data.favorite()).unwrap();
        });

        let sorter = gtk4::CustomSorter::new(move |obj1, obj2| {
            let info1 = obj1
                .downcast_ref::<crate::models::asset_data::AssetData>()
                .unwrap();
            let info2 = obj2
                .downcast_ref::<crate::models::asset_data::AssetData>()
                .unwrap();

            info1
                .name()
                .to_lowercase()
                .cmp(&info2.name().to_lowercase())
                .into()
        });

        self_.filter_model.set_model(Some(&self_.grid_model));
        let sorted_model = gtk4::SortListModel::new(Some(&self_.filter_model), Some(&sorter));
        let selection_model = gtk4::SingleSelection::new(Some(&sorted_model));
        selection_model.set_autoselect(false);
        selection_model.set_can_unselect(true);
        self_.asset_grid.set_model(Some(&selection_model));
        self_.asset_grid.set_factory(Some(&factory));

        selection_model.connect_selected_notify(clone!(@weak self as loggedin => move |model| {
            if let Some(a) = model.selected_item() {
                let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(&loggedin);
                let asset = a.downcast::<crate::models::asset_data::AssetData>().unwrap();
                let assets = self_.loaded_assets.borrow();
                if let Some(a) = assets.get(&asset.id()) {  self_.details.set_asset(a.clone()) }
            }
        }));

        self.fetch_assets();
    }

    /// Open asset based on a name from xdg-open
    fn open_asset(&self) {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        if let Some(id) = self.item() {
            let assets = self_.loaded_assets.borrow();
            if let Some(a) = assets.get(&id) {
                self_.details.set_asset(a.clone())
            }
        } else if let Some(product) = self.product() {
            let assets = self_.loaded_assets.borrow();
            let products = self_.asset_product_names.borrow();
            match products.get(&product) {
                Some(id) => {
                    if let Some(a) = assets.get(id) {
                        self_.details.set_asset(a.clone())
                    }
                }
                None => {
                    for prod in products.keys() {
                        if product.starts_with(prod) {
                            if let Some(id) = products.get(prod) {
                                if let Some(a) = assets.get(id) {
                                    self_.details.set_asset(a.clone())
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn flush_assets(&self) {
        let start = std::time::Instant::now();
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        if let Ok(mut vec) = self_.assets_pending.write() {
            if vec.is_empty() {
                return;
            }
            self_.grid_model.splice(0, 0, vec.as_slice());
            vec.clear();
        }
        self.open_asset();
        debug!("Finished flushing {:?}", start.elapsed());
    }

    pub fn bind_properties(&self) {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
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
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        self_.projects_category.set_logged_in(self);
        self_.assets_category.set_logged_in(self);
        self_.plugins_category.set_logged_in(self);
        self_.other_category.set_logged_in(self);
        self_.games_category.set_logged_in(self);
        self_.home_category.set_logged_in(self);
    }

    pub fn setup_actions(&self) {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);

        action!(
            self_.actions,
            "expand",
            clone!(@weak self as win => move |_, _| {
                    if let Ok(v) = win.property("sidebar-expanded") {
                    let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(&win);
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

        action!(
            self_.actions,
            "show_download_details",
            clone!(@weak self as win => move |_, _| {
                let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(&win);
                if let Some(w) = self_.window.get() {
                   w.show_download_manager()
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
        None
    }

    pub fn search(&self) -> Option<String> {
        if let Ok(value) = self.property("search") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn item(&self) -> Option<String> {
        if let Ok(value) = self.property("item") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn product(&self) -> Option<String> {
        if let Ok(value) = self.property("product") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn unselect_categories_except(&self) {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
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
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        let search = self.search();
        let filter_p = self.filter();
        if filter_p.is_none() && search.is_none() {
            self_.filter_model.set_filter(None::<&gtk4::CustomFilter>);
            return;
        }

        let filter = gtk4::CustomFilter::new(move |object| {
            let asset = object
                .downcast_ref::<crate::models::asset_data::AssetData>()
                .unwrap();
            (match &search {
                None => true,
                Some(se) => asset
                    .name()
                    .to_ascii_lowercase()
                    .contains(&se.to_ascii_lowercase()),
            }) && (match &filter_p {
                None => true,
                Some(f) => asset.check_category(f.clone()),
            })
        });
        self_.filter_model.set_filter(Some(&filter));
    }

    pub fn add_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo, image: &[u8]) {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        if let Some(categories) = &asset.categories {
            for category in categories {
                let mut cats = self_.categories.borrow_mut();
                if cats.insert(category.path.clone()) {
                    self.add_category(&category.path);
                }
            }
        };
        let mut assets = self_.loaded_assets.borrow_mut();
        let mut asset_products = self_.asset_product_names.borrow_mut();
        if match assets.get_mut(&asset.id) {
            None => {
                assets.insert(asset.id.clone(), asset.clone());
                if let Some(title) = asset.title.clone() {
                    let title: String = title
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
                        .collect();
                    let title: String = title.to_lowercase().replace(" ", "-");
                    asset_products.insert(title, asset.id.clone());
                }

                true
            }
            Some(a) => {
                if asset.id.eq(&a.id) {
                    // TODO: update asset if there are changes
                    debug!("Duplicate asset: {}", asset.id);
                    false
                } else {
                    assets.insert(asset.id.clone(), asset.clone());
                    if let Some(title) = asset.title.clone() {
                        let title: String = title
                            .chars()
                            .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
                            .collect();
                        let title: String = title.to_lowercase().replace(" ", "-");
                        asset_products.insert(title, asset.id.clone());
                    }
                    true
                }
            }
        } {
            let data = crate::models::asset_data::AssetData::new(asset, image);
            if let Ok(mut vec) = self_.assets_pending.write() {
                vec.push(data.upcast());
            }
        }
    }

    fn add_category(&self, path: &str) {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        let parts = path.split('/').collect::<Vec<&str>>();
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

    pub fn load_thumbnail(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        if let Some(window) = self.main_window() {
            let win_ = window.data();
            let sender = win_.model.borrow().sender.clone();
            match asset.thumbnail() {
                None => {
                    sender
                        .send(crate::ui::messages::Msg::ProcessAssetThumbnail(
                            asset.clone(),
                            vec![],
                        ))
                        .unwrap();
                }
                Some(t) => {
                    let cache_dir = self_.settings.string("cache-directory").to_string();
                    let mut cache_path = PathBuf::from(cache_dir);
                    cache_path.push("images");
                    let name = Path::new(t.url.path()).extension().and_then(OsStr::to_str);
                    cache_path.push(format!("{}.{}", t.md5, name.unwrap_or(".png")));
                    let asset = asset.clone();
                    self_.image_load_pool.execute(move || {
                        if let Ok(w) = crate::RUNNING.read() {
                            if !*w {
                                return;
                            }
                        }
                        match File::open(cache_path.as_path()) {
                            Ok(mut f) => {
                                fs::create_dir_all(&cache_path.parent().unwrap()).unwrap();
                                let metadata = fs::metadata(&cache_path.as_path())
                                    .expect("unable to read metadata");
                                let mut buffer = vec![0; metadata.len() as usize];
                                f.read_exact(&mut buffer).expect("buffer overflow");
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
                                                asset.clone(),
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
                                sender
                                    .send(crate::ui::messages::Msg::DownloadImage(t, asset.clone()))
                                    .unwrap();
                            }
                        };
                    })
                }
            }
        }
    }

    fn main_window(&self) -> Option<&crate::window::EpicAssetManagerWindow> {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        match self_.window.get() {
            Some(window) => Some(&(*window)),
            None => None,
        }
    }

    pub fn fetch_assets(&self) {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        if let Some(window) = self.main_window() {
            let win_ = window.data();
            let cache_dir = self_.settings.string("cache-directory").to_string();
            let cache_path = PathBuf::from(cache_dir);
            debug!("Fetching assets");
            if cache_path.is_dir() {
                debug!("Checking cache");
                for entry in std::fs::read_dir(cache_path).unwrap() {
                    let sender = win_.model.borrow().sender.clone();
                    self_.asset_load_pool.execute(move || {
                        // Load assets from cache

                        if let Ok(w) = crate::RUNNING.read() {
                            if !*w {
                                return;
                            }
                        }
                        let mut asset_file = entry.unwrap().path();
                        asset_file.push("asset_info.json");
                        if asset_file.exists() {
                            if let Ok(mut f) = std::fs::File::open(asset_file.as_path()) {
                                let mut buffer = String::new();
                                f.read_to_string(&mut buffer).unwrap();
                                if let Ok(asset) = json5::from_str::<
                                    egs_api::api::types::asset_info::AssetInfo,
                                >(&buffer)
                                {
                                    sender
                                        .send(crate::ui::messages::Msg::ProcessAssetInfo(asset))
                                        .unwrap();
                                }
                            };
                        }
                    });
                }
            };
            let mut eg = win_.model.borrow().epic_games.borrow().clone();
            let sender = win_.model.borrow().sender.clone();
            self_.asset_load_pool.execute(move || {
                let assets = tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(eg.list_assets());
                for asset in assets {
                    sender
                        .send(crate::ui::messages::Msg::ProcessEpicAsset(asset))
                        .unwrap();
                }
            });
            glib::idle_add_local(clone!(@weak self as obj => @default-panic, move || {
                obj.flush_assets();
                let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(&obj);
                glib::Continue((self_.asset_load_pool.queued_count() +
                    self_.asset_load_pool.active_count() +
                    self_.image_load_pool.queued_count() +
                    self_.image_load_pool.active_count()) > 0)
            }));
            glib::timeout_add_seconds_local(
                1,
                clone!(@weak self as obj => @default-panic, move || {
                    let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(&obj);
                    if let Ok(a) = self_.assets_pending.read() {
                        if a.len() > 0 {
                            glib::idle_add_local(clone!(@weak obj => @default-panic, move || {
                                obj.flush_assets();
                                glib::Continue(false)
                            }));
                        }
                    }
                    glib::Continue(true)
                }),
            );
        }
    }

    pub(crate) fn process_epic_asset(
        &self,
        epic_asset: &egs_api::api::types::epic_asset::EpicAsset,
    ) {
        let self_: &imp::EpicLibraryBox = imp::EpicLibraryBox::from_instance(self);
        if let Some(window) = self.main_window() {
            let win_ = window.data();
            let mut cache_dir = PathBuf::from(self_.settings.string("cache-directory").to_string());
            cache_dir.push(&epic_asset.catalog_item_id);
            let mut cache_dir_c = cache_dir.clone();
            let ea = epic_asset.clone();

            self_.asset_load_pool.execute(move || {
                cache_dir_c.push("epic_asset.json");
                fs::create_dir_all(cache_dir_c.parent().unwrap()).unwrap();
                if let Ok(mut asset_file) = File::create(cache_dir_c.as_path()) {
                    asset_file
                        .write_all(json5::to_string(&ea).unwrap().as_bytes().as_ref())
                        .unwrap();
                }
            });

            let mut eg = win_.model.borrow().epic_games.borrow().clone();
            let sender = win_.model.borrow().sender.clone();
            let mut cache_dir_c = cache_dir;
            let epic_asset = epic_asset.clone();
            self_.asset_load_pool.execute(move || {
                if let Ok(w) = crate::RUNNING.read() {
                    if !*w {
                        return;
                    }
                }
                if let Some(asset) = tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(eg.asset_info(epic_asset.clone()))
                {
                    // TODO: Check with already added assets to see if it needs updating
                    cache_dir_c.push("asset_info.json");
                    fs::create_dir_all(cache_dir_c.parent().unwrap()).unwrap();
                    if let Ok(mut asset_file) = File::create(cache_dir_c.as_path()) {
                        asset_file
                            .write_all(json5::to_string(&asset).unwrap().as_bytes().as_ref())
                            .unwrap();
                    }
                    sender
                        .send(crate::ui::messages::Msg::ProcessAssetInfo(asset))
                        .unwrap();
                }
            });
        }
    }
}
