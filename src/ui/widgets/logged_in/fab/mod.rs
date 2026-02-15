use glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use log::{debug, error, warn};
use std::path::PathBuf;

pub mod version_dialog;

pub mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::ui::widgets::logged_in::library::asset_detail::EpicAssetDetails;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::gio::ListStore;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use std::collections::{BTreeSet, HashSet};
    use threadpool::ThreadPool;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/fab.ui")]
    pub struct FabLibraryBox {
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub details: OnceCell<EpicAssetDetails>,
        #[template_child]
        pub fab_grid: TemplateChild<gtk4::GridView>,
        #[template_child]
        pub fab_search: TemplateChild<gtk4::SearchEntry>,
        #[template_child]
        pub category_dropdown: TemplateChild<gtk4::DropDown>,
        #[template_child]
        pub downloaded_filter: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub favorites_filter: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub count_label: TemplateChild<gtk4::Label>,
        #[template_child]
        pub refresh_progress: TemplateChild<gtk4::ProgressBar>,
        pub grid_model: ListStore,
        pub filter_model: gtk4::FilterListModel,
        pub search: RefCell<Option<String>>,
        /// Tracks asset IDs already in the grid to avoid duplicates on refresh
        pub known_asset_ids: RefCell<HashSet<String>>,
        /// Sorted set of known category names for the dropdown
        pub category_names: RefCell<BTreeSet<String>>,
        /// Parallel index: position 0 = "" (All), then sorted category names
        pub category_filter_names: RefCell<Vec<String>>,
        pub image_load_pool: ThreadPool,
        pub settings: gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FabLibraryBox {
        const NAME: &'static str = "FabLibraryBox";
        type Type = super::FabLibraryBox;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                details: OnceCell::new(),
                fab_grid: TemplateChild::default(),
                fab_search: TemplateChild::default(),
                category_dropdown: TemplateChild::default(),
                downloaded_filter: TemplateChild::default(),
                favorites_filter: TemplateChild::default(),
                count_label: TemplateChild::default(),
                refresh_progress: TemplateChild::default(),
                grid_model: ListStore::new::<crate::models::fab_data::FabData>(),
                filter_model: gtk4::FilterListModel::new(
                    None::<gtk4::gio::ListStore>,
                    None::<gtk4::CustomFilter>,
                ),
                search: RefCell::new(None),
                known_asset_ids: RefCell::new(HashSet::new()),
                category_names: RefCell::new(BTreeSet::new()),
                category_filter_names: RefCell::new(vec![String::new()]),
                image_load_pool: ThreadPool::with_name("fab_image_pool".to_string(), 10),
                settings: gio::Settings::new(crate::config::APP_ID),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FabLibraryBox {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_widgets();
        }
    }

    impl WidgetImpl for FabLibraryBox {}
    impl BoxImpl for FabLibraryBox {}
}

glib::wrapper! {
    pub struct FabLibraryBox(ObjectSubclass<imp::FabLibraryBox>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl Default for FabLibraryBox {
    fn default() -> Self {
        Self::new()
    }
}

impl FabLibraryBox {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        self.setup_grid();
        self.load_cached_fab_assets();
        self.fetch_fab_assets();
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_ = self.imp();
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn set_details(
        &self,
        details: &crate::ui::widgets::logged_in::library::asset_detail::EpicAssetDetails,
    ) {
        let self_ = self.imp();
        if self_.details.get().is_some() {
            return;
        }
        self_.details.set(details.clone()).unwrap();
    }

    fn main_window(&self) -> Option<&crate::window::EpicAssetManagerWindow> {
        self.imp().window.get()
    }

    fn setup_widgets(&self) {
        let self_ = self.imp();

        let cat_model = gtk4::StringList::new(&["All"]);
        self_.category_dropdown.set_model(Some(&cat_model));
        self_.category_dropdown.set_selected(0);

        self_.fab_search.connect_search_changed(clone!(
            #[weak(rename_to=fab)]
            self,
            move |entry| {
                let text = entry.text();
                let search = if text.is_empty() {
                    None
                } else {
                    Some(text.to_string())
                };
                fab.imp().search.replace(search);
                fab.update_filter();
            }
        ));

        self_.category_dropdown.connect_selected_notify(clone!(
            #[weak(rename_to=fab)]
            self,
            move |_| {
                fab.update_filter();
            }
        ));

        self_.downloaded_filter.connect_toggled(clone!(
            #[weak(rename_to=fab)]
            self,
            move |_| {
                fab.update_filter();
            }
        ));

        self_.favorites_filter.connect_toggled(clone!(
            #[weak(rename_to=fab)]
            self,
            move |_| {
                fab.update_filter();
            }
        ));
    }

    fn setup_grid(&self) {
        let self_ = self.imp();

        let factory = gtk4::SignalListItemFactory::new();
        factory.connect_setup(clone!(
            #[weak(rename_to=fab)]
            self,
            move |_factory, item| {
                let row = crate::ui::widgets::logged_in::library::asset::EpicAsset::new();
                let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
                item.set_child(Some(&row));

                row.connect_local(
                    "tile-clicked",
                    false,
                    clone!(
                        #[weak]
                        fab,
                        #[upgrade_or]
                        None,
                        move |values| {
                            let asset_widget = values[0]
                                .get::<crate::ui::widgets::logged_in::library::asset::EpicAsset>()
                                .unwrap();
                            fab.handle_tile_clicked(&asset_widget);
                            None
                        }
                    ),
                );

                row.connect_local(
                    "download-requested",
                    false,
                    clone!(
                        #[weak]
                        fab,
                        #[upgrade_or]
                        None,
                        move |values| {
                            let asset_widget = values[0]
                                .get::<crate::ui::widgets::logged_in::library::asset::EpicAsset>()
                                .unwrap();
                            fab.handle_download_requested(&asset_widget);
                            None
                        }
                    ),
                );
            }
        ));

        factory.connect_bind(move |_, list_item| {
            let item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            if let Some(data) = item.item() {
                if let Some(fab_data) =
                    data.downcast_ref::<crate::models::fab_data::FabData>()
                {
                    if let Some(child) = item.child() {
                        if let Some(asset) = child.downcast_ref::<crate::ui::widgets::logged_in::library::asset::EpicAsset>() {
                            asset.set_fab_data(fab_data);
                        }
                    }
                }
            }
        });

        self_.filter_model.set_model(Some(&self_.grid_model));

        let selection_model = gtk4::SingleSelection::builder()
            .model(&self_.filter_model)
            .autoselect(false)
            .can_unselect(true)
            .build();

        self_.fab_grid.set_model(Some(&selection_model));
        self_.fab_grid.set_factory(Some(&factory));
    }

    fn update_filter(&self) {
        let self_ = self.imp();
        let search = self_.search.borrow().clone();
        let downloaded_only = self_.downloaded_filter.is_active();
        let favorites_only = self_.favorites_filter.is_active();

        let selected_cat = self_.category_dropdown.selected();
        let category_filter: Option<String> =
            if selected_cat == 0 || selected_cat == gtk4::INVALID_LIST_POSITION {
                None
            } else {
                self_
                    .category_filter_names
                    .borrow()
                    .get(selected_cat as usize)
                    .filter(|n| !n.is_empty())
                    .cloned()
            };

        if search.is_none() && !downloaded_only && !favorites_only && category_filter.is_none() {
            self_.filter_model.set_filter(None::<&gtk4::CustomFilter>);
            self.update_count();
            return;
        }

        let filter = gtk4::CustomFilter::new(move |obj| {
            if let Some(data) = obj.downcast_ref::<crate::models::fab_data::FabData>() {
                let matches_search = search.as_ref().map_or(true, |s| {
                    data.name().to_lowercase().contains(&s.to_lowercase())
                });
                let matches_downloaded = !downloaded_only || data.downloaded();
                let matches_favorites = !favorites_only || data.favorite();
                let matches_category = category_filter
                    .as_ref()
                    .map_or(true, |c| data.check_category(c));

                return matches_search
                    && matches_downloaded
                    && matches_favorites
                    && matches_category;
            }
            false
        });

        self_.filter_model.set_filter(Some(&filter));
        self.update_count();
    }

    fn update_count(&self) {
        let self_ = self.imp();
        let count = self_.filter_model.n_items();
        self_.count_label.set_label(&format!("{} assets", count));
    }

    fn add_asset_categories(&self, asset: &egs_api::api::types::fab_library::FabAsset) {
        let self_ = self.imp();
        let mut names = self_.category_names.borrow_mut();
        let mut changed = false;

        for category in &asset.categories {
            if let Some(name) = &category.name {
                if !name.is_empty() && names.insert(name.clone()) {
                    changed = true;
                }
            }
        }

        if changed {
            drop(names);
            self.rebuild_category_dropdown();
        }
    }

    fn rebuild_category_dropdown(&self) {
        let self_ = self.imp();
        let prev_selected = self_.category_dropdown.selected();
        let prev_name = self_
            .category_filter_names
            .borrow()
            .get(prev_selected as usize)
            .cloned()
            .unwrap_or_default();

        let names = self_.category_names.borrow();

        let model = gtk4::StringList::new(&["All"]);
        let mut filter_names = vec![String::new()];

        for name in names.iter() {
            model.append(name);
            filter_names.push(name.clone());
        }

        let new_selected = filter_names
            .iter()
            .position(|n| n == &prev_name)
            .unwrap_or(0) as u32;

        self_.category_filter_names.replace(filter_names);
        self_.category_dropdown.set_model(Some(&model));
        self_.category_dropdown.set_selected(new_selected);
    }

    fn fab_cache_dir(cache_dir: &str) -> PathBuf {
        let mut path = PathBuf::from(cache_dir);
        path.push("fab");
        path
    }

    fn cache_fab_asset(asset: &egs_api::api::types::fab_library::FabAsset, cache_dir: &str) {
        let mut cache_path = Self::fab_cache_dir(cache_dir);
        cache_path.push(&asset.asset_id);
        if std::fs::create_dir_all(&cache_path).is_err() {
            warn!("Failed to create FAB cache directory: {:?}", cache_path);
            return;
        }
        cache_path.push("fab_asset.json");
        match std::fs::File::create(&cache_path) {
            Ok(file) => {
                if let Err(e) = serde_json::to_writer(file, asset) {
                    warn!("Failed to write FAB cache for {}: {}", asset.asset_id, e);
                }
            }
            Err(e) => {
                warn!("Failed to create FAB cache file {:?}: {}", cache_path, e);
            }
        }
    }

    fn load_cached_fab_assets(&self) {
        let self_ = self.imp();
        let cache_dir = self_.settings.string("cache-directory").to_string();
        let fab_cache = Self::fab_cache_dir(&cache_dir);

        if !fab_cache.is_dir() {
            debug!(
                "No FAB cache directory at {:?}, skipping cache load",
                fab_cache
            );
            return;
        }

        if let Some(window) = self.main_window() {
            let win_ = window.imp();
            let sender = win_.model.borrow().sender.clone();

            self_.refresh_progress.set_visible(true);
            self_
                .refresh_progress
                .set_tooltip_text(Some("Loading from cache"));

            self_.image_load_pool.execute(move || {
                let entries = match std::fs::read_dir(&fab_cache) {
                    Ok(e) => e,
                    Err(e) => {
                        warn!("Failed to read FAB cache directory: {}", e);
                        return;
                    }
                };

                for entry in entries.flatten() {
                    if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }

                    let mut asset_file = entry.path();
                    if !asset_file.is_dir() {
                        continue;
                    }
                    asset_file.push("fab_asset.json");
                    if !asset_file.exists() {
                        continue;
                    }

                    let file = match std::fs::File::open(&asset_file) {
                        Ok(f) => f,
                        Err(e) => {
                            warn!("Failed to open cached FAB asset {:?}: {}", asset_file, e);
                            continue;
                        }
                    };

                    let asset: egs_api::api::types::fab_library::FabAsset =
                        match serde_json::from_reader(file) {
                            Ok(a) => a,
                            Err(e) => {
                                warn!("Failed to parse cached FAB asset {:?}: {}", asset_file, e);
                                continue;
                            }
                        };

                    let texture = Self::load_fab_thumbnail(&asset, &cache_dir);
                    sender
                        .send_blocking(crate::ui::messages::Msg::ProcessFabAsset(asset, texture))
                        .unwrap();
                }

                sender
                    .send_blocking(crate::ui::messages::Msg::FlushFabAssets)
                    .unwrap();
            });
        }
    }

    pub fn fetch_fab_assets(&self) {
        let self_ = self.imp();
        self_.refresh_progress.set_visible(true);
        self_.refresh_progress.pulse();

        if let Some(window) = self.main_window() {
            let win_ = window.imp();
            let mut eg = win_.model.borrow().epic_games.borrow().clone();
            let sender = win_.model.borrow().sender.clone();
            let cache_dir = self_.settings.string("cache-directory").to_string();

            let account_id = eg.user_details().account_id.clone();
            if account_id.is_none() {
                error!("No account_id available for FAB library fetch");
                self_.refresh_progress.set_visible(false);
                return;
            }
            let account_id = account_id.unwrap();

            debug!("Fetching FAB library from API for account {}", account_id);

            self_.image_load_pool.execute(move || {
                if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }

                let fab_library = crate::RUNTIME.block_on(eg.fab_library_items(account_id));

                if let Some(library) = fab_library {
                    debug!("Got {} FAB assets from API", library.results.len());
                    for asset in &library.results {
                        if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                            return;
                        }

                        Self::cache_fab_asset(asset, &cache_dir);
                        let texture = Self::load_fab_thumbnail(asset, &cache_dir);
                        sender
                            .send_blocking(crate::ui::messages::Msg::ProcessFabAsset(
                                asset.clone(),
                                texture,
                            ))
                            .unwrap();
                    }
                } else {
                    error!("Failed to fetch FAB library from API");
                }

                sender
                    .send_blocking(crate::ui::messages::Msg::FlushFabAssets)
                    .unwrap();
            });
        }
    }

    fn load_fab_thumbnail(
        asset: &egs_api::api::types::fab_library::FabAsset,
        cache_dir: &str,
    ) -> Option<gtk4::gdk::Texture> {
        let image = asset.images.first()?;
        let url = &image.url;

        let mut cache_path = PathBuf::from(cache_dir);
        cache_path.push("fab_images");

        let cache_key = image.md5.as_deref().unwrap_or(&asset.asset_id);
        let extension = url
            .rsplit('.')
            .next()
            .filter(|e| e.len() <= 4)
            .unwrap_or("png");
        cache_path.push(format!("{}.{}", cache_key, extension));

        if cache_path.exists() {
            return gtk4::gdk::Texture::from_file(&gio::File::for_path(&cache_path)).ok();
        }

        if let Ok(response) = reqwest::blocking::get(url) {
            if let Ok(bytes) = response.bytes() {
                std::fs::create_dir_all(cache_path.parent().unwrap()).ok();
                if let Ok(mut file) = std::fs::File::create(&cache_path) {
                    use std::io::Write;
                    file.write_all(&bytes).ok();
                }
                return gtk4::gdk::Texture::from_file(&gio::File::for_path(&cache_path)).ok();
            }
        }

        None
    }

    pub fn add_fab_asset(
        &self,
        asset: &egs_api::api::types::fab_library::FabAsset,
        image: Option<gtk4::gdk::Texture>,
    ) {
        let self_ = self.imp();
        if !self_
            .known_asset_ids
            .borrow_mut()
            .insert(asset.asset_id.clone())
        {
            return;
        }
        self.add_asset_categories(asset);
        let data = crate::models::fab_data::FabData::new(asset, image);
        self_.grid_model.append(&data);
        self.update_count();
    }

    pub fn flush_fab_assets(&self) {
        let self_ = self.imp();
        self_.refresh_progress.set_visible(false);
        self.update_count();
    }

    pub fn clear(&self) {
        let self_ = self.imp();
        self_.grid_model.remove_all();
        self_.known_asset_ids.borrow_mut().clear();
        self_.category_names.borrow_mut().clear();
        self_.category_filter_names.replace(vec![String::new()]);
        let model = gtk4::StringList::new(&["All"]);
        self_.category_dropdown.set_model(Some(&model));
        self_.category_dropdown.set_selected(0);
        self.update_count();
    }

    fn handle_tile_clicked(
        &self,
        asset_widget: &crate::ui::widgets::logged_in::library::asset::EpicAsset,
    ) {
        let self_ = self.imp();
        if let Some(fab_data) = asset_widget.imp().fab_data.borrow().as_ref() {
            if let Some(asset) = fab_data.imp().asset.borrow().as_ref() {
                if let Some(details) = self_.details.get() {
                    details.set_fab_asset(asset);
                }
            }
        }
    }

    fn handle_download_requested(
        &self,
        asset_widget: &crate::ui::widgets::logged_in::library::asset::EpicAsset,
    ) {
        let self_ = self.imp();
        if let Some(fab_data) = asset_widget.imp().fab_data.borrow().as_ref() {
            if let Some(asset) = fab_data.imp().asset.borrow().as_ref() {
                if let Some(details) = self_.details.get() {
                    details.set_fab_asset(asset);
                    if !asset.project_versions.is_empty() {
                        details.open_fab_version_dialog(asset);
                    }
                }
            }
        }
    }

    pub fn run_refresh(&self) {
        self.clear();
        self.fetch_fab_assets();
    }
}
