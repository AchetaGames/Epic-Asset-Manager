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
        pub browse_toggle: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub category_dropdown: TemplateChild<gtk4::DropDown>,
        #[template_child]
        pub downloaded_filter: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub favorites_filter: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub free_filter: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub on_sale_filter: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub load_more_button: TemplateChild<gtk4::Button>,
        #[template_child]
        pub count_label: TemplateChild<gtk4::Label>,
        #[template_child]
        pub refresh_progress: TemplateChild<gtk4::ProgressBar>,
        pub grid_model: ListStore,
        pub browse_model: ListStore,
        pub filter_model: gtk4::FilterListModel,
        pub browse_filter_model: gtk4::FilterListModel,
        pub browse_mode: RefCell<bool>,
        pub browse_category: RefCell<Option<String>>,
        pub taxonomy_loaded: std::cell::Cell<bool>,
        pub fab_taxonomy: RefCell<Option<Vec<egs_api::api::types::fab_taxonomy::FabTagGroup>>>,
        pub browse_cursor: RefCell<Option<String>>,
        pub browse_known_ids: RefCell<HashSet<String>>,
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
                browse_toggle: TemplateChild::default(),
                category_dropdown: TemplateChild::default(),
                downloaded_filter: TemplateChild::default(),
                favorites_filter: TemplateChild::default(),
                free_filter: TemplateChild::default(),
                on_sale_filter: TemplateChild::default(),
                load_more_button: TemplateChild::default(),
                count_label: TemplateChild::default(),
                refresh_progress: TemplateChild::default(),
                grid_model: ListStore::new::<crate::models::fab_data::FabData>(),
                browse_model: ListStore::new::<crate::models::fab_data::FabData>(),
                filter_model: gtk4::FilterListModel::new(
                    None::<gtk4::gio::ListStore>,
                    None::<gtk4::CustomFilter>,
                ),
                browse_filter_model: gtk4::FilterListModel::new(
                    None::<gtk4::gio::ListStore>,
                    None::<gtk4::CustomFilter>,
                ),
                browse_mode: RefCell::new(false),
                browse_category: RefCell::new(None),
                taxonomy_loaded: std::cell::Cell::new(false),
                fab_taxonomy: RefCell::new(None),
                browse_cursor: RefCell::new(None),
                browse_known_ids: RefCell::new(HashSet::new()),
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
                if *fab.imp().browse_mode.borrow() {
                    return;
                }
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

        self_.fab_search.connect_activate(clone!(
            #[weak(rename_to=fab)]
            self,
            move |entry| {
                if *fab.imp().browse_mode.borrow() {
                    let text = entry.text();
                    fab.search_marketplace(&text);
                }
            }
        ));

        self_.browse_toggle.connect_toggled(clone!(
            #[weak(rename_to=fab)]
            self,
            move |toggle| {
                let browse = toggle.is_active();
                fab.set_browse_mode(browse);
            }
        ));

        self_.load_more_button.connect_clicked(clone!(
            #[weak(rename_to=fab)]
            self,
            move |_| {
                fab.load_more_browse_results();
            }
        ));

        self_.category_dropdown.connect_selected_notify(clone!(
            #[weak(rename_to=fab)]
            self,
            move |dropdown| {
                let self_ = fab.imp();
                if *self_.browse_mode.borrow() {
                    let idx = dropdown.selected() as usize;
                    let slug = self_
                        .category_filter_names
                        .borrow()
                        .get(idx)
                        .cloned()
                        .unwrap_or_default();
                    let category = if slug.is_empty() { None } else { Some(slug) };
                    self_.browse_category.replace(category);
                    self_.browse_cursor.replace(None);
                    self_.browse_known_ids.borrow_mut().clear();
                    self_.browse_model.remove_all();
                    fab.fetch_browse_results(None);
                } else {
                    fab.update_filter();
                }
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

        self_.free_filter.connect_toggled(clone!(
            #[weak(rename_to=fab)]
            self,
            move |_| {
                let self_ = fab.imp();
                self_.browse_cursor.replace(None);
                self_.browse_known_ids.borrow_mut().clear();
                self_.browse_model.remove_all();
                fab.fetch_browse_results(None);
            }
        ));

        self_.on_sale_filter.connect_toggled(clone!(
            #[weak(rename_to=fab)]
            self,
            move |_| {
                let self_ = fab.imp();
                self_.browse_cursor.replace(None);
                self_.browse_known_ids.borrow_mut().clear();
                self_.browse_model.remove_all();
                fab.fetch_browse_results(None);
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

                row.connect_local(
                    "add-to-library-requested",
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
                            fab.handle_add_to_library(&asset_widget);
                            None
                        }
                    ),
                );
            }
        ));

        factory.connect_bind(move |_, list_item| {
            let item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            if let Some(data) = item.item() {
                if let Some(child) = item.child() {
                    if let Some(asset) = child
                        .downcast_ref::<crate::ui::widgets::logged_in::library::asset::EpicAsset>(
                    ) {
                        if let Some(fab_data) =
                            data.downcast_ref::<crate::models::fab_data::FabData>()
                        {
                            asset.set_fab_data(fab_data);
                        }
                    }
                }
            }
        });

        self_.filter_model.set_model(Some(&self_.grid_model));
        self_
            .browse_filter_model
            .set_model(Some(&self_.browse_model));
        self.bind_grid_model(&self_.filter_model);
        self_.fab_grid.set_factory(Some(&factory));
    }

    fn bind_grid_model(&self, model: &gtk4::FilterListModel) {
        let self_ = self.imp();
        let selection_model = gtk4::SingleSelection::builder()
            .model(model)
            .autoselect(false)
            .can_unselect(true)
            .build();

        self_.fab_grid.set_model(Some(&selection_model));
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

        if *self_.browse_mode.borrow() {
            self_
                .browse_filter_model
                .set_filter(None::<&gtk4::CustomFilter>);
            self.update_count();
            return;
        }

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
        let count = if *self_.browse_mode.borrow() {
            self_.browse_filter_model.n_items()
        } else {
            self_.filter_model.n_items()
        };
        self_.count_label.set_label(&format!("{} assets", count));
    }

    fn search_marketplace(&self, query: &str) {
        let self_ = self.imp();
        let search = if query.is_empty() {
            None
        } else {
            Some(query.to_string())
        };
        self_.search.replace(search);

        // Server-side search: clear existing results and re-fetch with query
        self_.browse_cursor.replace(None);
        self_.browse_known_ids.borrow_mut().clear();
        self_.browse_model.remove_all();
        self.fetch_browse_results(None);
    }

    fn set_browse_mode(&self, browse: bool) {
        let self_ = self.imp();
        self_.browse_mode.replace(browse);
        if browse {
            self.load_fab_taxonomy();
            let cached_groups = self_.fab_taxonomy.borrow().clone();
            if let Some(groups) = cached_groups {
                self.apply_fab_taxonomy(groups);
            }
            self_.browse_cursor.replace(None);
            self_.browse_known_ids.borrow_mut().clear();
            self_.browse_model.remove_all();
            self_.downloaded_filter.set_visible(false);
            self_.favorites_filter.set_visible(false);
            self_.free_filter.set_visible(true);
            self_.on_sale_filter.set_visible(true);
            self_.category_dropdown.set_visible(true);
            self.fetch_browse_results(None);
        } else {
            self_.browse_category.replace(None);
            self_.load_more_button.set_visible(false);
            self_.downloaded_filter.set_visible(true);
            self_.favorites_filter.set_visible(true);
            self_.free_filter.set_visible(false);
            self_.on_sale_filter.set_visible(false);
            self.rebuild_category_dropdown();
        }
        self.bind_grid_model(if browse {
            &self_.browse_filter_model
        } else {
            &self_.filter_model
        });
        self.update_filter();
        self.update_count();
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

    fn load_fab_taxonomy(&self) {
        let self_ = self.imp();
        if self_.taxonomy_loaded.get() {
            return;
        }
        self_.taxonomy_loaded.set(true);

        if let Some(window) = self.main_window() {
            let win_ = window.imp();
            let eg = win_.model.borrow().epic_games.borrow().clone();
            let sender = win_.model.borrow().sender.clone();

            self_.image_load_pool.execute(move || {
                if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                if let Some(groups) = crate::RUNTIME.block_on(eg.fab_tag_groups()) {
                    let _ =
                        sender.send_blocking(crate::ui::messages::Msg::FabTaxonomyLoaded(groups));
                }
            });
        }
    }

    pub fn apply_fab_taxonomy(&self, groups: Vec<egs_api::api::types::fab_taxonomy::FabTagGroup>) {
        let self_ = self.imp();
        self_.fab_taxonomy.replace(Some(groups));
        if !*self_.browse_mode.borrow() {
            return;
        }

        let model = gtk4::StringList::new(&["All"]);
        let mut filter_names = vec![String::new()];

        if let Some(groups) = self_.fab_taxonomy.borrow().as_ref() {
            for group in groups {
                if let Some(tags) = &group.tags {
                    for tag in tags {
                        if let Some(name) = &tag.name {
                            model.append(name);
                            filter_names.push(tag.slug.clone().unwrap_or_default());
                        }
                    }
                }
            }
        }

        self_.category_filter_names.replace(filter_names);
        self_.category_dropdown.set_model(Some(&model));
        self_.category_dropdown.set_selected(0);
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

    pub fn refresh_fab_asset(&self, id: &str) {
        let self_ = self.imp();
        for i in 0..self_.grid_model.n_items() {
            if let Some(obj) = self_.grid_model.item(i) {
                if let Some(data) = obj.downcast_ref::<crate::models::fab_data::FabData>() {
                    if data.id() == id {
                        data.refresh();
                        break;
                    }
                }
            }
        }
        self.update_filter();
    }

    pub fn flush_fab_assets(&self) {
        let self_ = self.imp();
        self_.refresh_progress.set_visible(false);
        self.update_count();
    }

    pub fn add_fab_browse_result(
        &self,
        asset: &egs_api::api::types::fab_library::FabAsset,
        image: Option<gtk4::gdk::Texture>,
        price_label: &str,
    ) {
        let self_ = self.imp();
        if !self_
            .browse_known_ids
            .borrow_mut()
            .insert(asset.asset_id.clone())
        {
            return;
        }
        let data = crate::models::fab_data::FabData::new_browse(asset, image, price_label);
        self_.browse_model.append(&data);
        self.update_count();
    }

    pub fn flush_fab_browse_results(&self, cursor: Option<String>) {
        let self_ = self.imp();
        self_.browse_cursor.replace(cursor);
        self_.refresh_progress.set_visible(false);
        let show_load_more =
            *self_.browse_mode.borrow() && self_.browse_cursor.borrow().as_ref().is_some();
        self_.load_more_button.set_visible(show_load_more);
        self.update_count();
    }

    pub fn fetch_browse_results(&self, cursor: Option<String>) {
        let self_ = self.imp();
        self_.refresh_progress.set_visible(true);

        if let Some(window) = self.main_window() {
            let win_ = window.imp();
            let eg = win_.model.borrow().epic_games.borrow().clone();
            let sender = win_.model.borrow().sender.clone();
            let search_text = self_.search.borrow().clone();
            let browse_cat = self_.browse_category.borrow().clone();
            let is_free = self_.free_filter.is_active();
            let on_sale = self_.on_sale_filter.is_active();

            self_.image_load_pool.execute(move || {
                if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }

                use egs_api::api::types::fab_search::FabSearchParams;
                let mut params = FabSearchParams::default();
                params.channels = Some("unreal-engine".to_string());
                params.count = Some(40);
                params.sort_by = Some("-createdAt".to_string());
                params.cursor = cursor;
                params.q = search_text;
                params.categories = browse_cat;
                if is_free {
                    params.is_free = Some(true);
                }
                if on_sale {
                    params.min_discount_percentage = Some(1);
                }

                match crate::RUNTIME.block_on(eg.try_fab_search(&params)) {
                    Ok(results) => {
                        for listing in results.results {
                            if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                                return;
                            }

                            let mut asset = egs_api::api::types::fab_library::FabAsset::default();
                            asset.asset_id = listing.uid.clone();
                            asset.title = listing.title.clone().unwrap_or_default();
                            asset.url = format!("https://www.fab.com/listings/{}", listing.uid);
                            if let Some(cat) = listing.category.as_ref() {
                                asset.categories =
                                    vec![egs_api::api::types::fab_library::Category {
                                        id: String::new(),
                                        name: cat.name.clone(),
                                    }];
                            }
                            if let Some(thumbs) = listing.thumbnails.as_ref() {
                                if let Some(url) =
                                    thumbs.iter().find_map(|thumb| thumb.media_url.clone())
                                {
                                    asset.images = vec![egs_api::api::types::fab_library::Image {
                                        height: String::new(),
                                        md5: None,
                                        type_field: "thumbnail".to_string(),
                                        uploaded_date: String::new(),
                                        url: url.clone(),
                                        width: String::new(),
                                    }];
                                }
                            }

                            let texture = listing.thumbnails.as_ref().and_then(|thumbs| {
                                thumbs.iter().find_map(|thumb| {
                                    thumb.media_url.clone().and_then(|url| {
                                        reqwest::blocking::get(url)
                                            .ok()
                                            .and_then(|response| response.bytes().ok())
                                            .and_then(|bytes| {
                                                let bytes = glib::Bytes::from_owned(bytes.to_vec());
                                                gtk4::gdk::Texture::from_bytes(&bytes).ok()
                                            })
                                    })
                                })
                            });

                            let price_label = if listing.is_free == Some(true) {
                                "Free".to_string()
                            } else if let Some(price_val) = &listing.starting_price {
                                price_val
                                    .get("price")
                                    .and_then(|p| p.as_f64())
                                    .map(|p| format!("${:.2}", p))
                                    .unwrap_or_else(|| "View on Fab".to_string())
                            } else {
                                "View on Fab".to_string()
                            };

                            sender
                                .send_blocking(crate::ui::messages::Msg::ProcessFabBrowseResult(
                                    asset,
                                    texture,
                                    price_label,
                                ))
                                .unwrap();
                        }

                        let next_cursor = results.cursors.as_ref().and_then(|c| c.next.clone());
                        sender
                            .send_blocking(crate::ui::messages::Msg::FlushFabBrowseResults(
                                next_cursor,
                            ))
                            .unwrap();
                    }
                    Err(e) => {
                        error!("Failed to fetch FAB browse results: {}", e);
                        sender
                            .send_blocking(crate::ui::messages::Msg::FlushFabBrowseResults(None))
                            .unwrap();
                    }
                }
            });
        }
    }

    pub fn load_more_browse_results(&self) {
        let cursor = self.imp().browse_cursor.borrow().clone();
        self.fetch_browse_results(cursor);
    }

    pub fn clear_browse_results(&self) {
        let self_ = self.imp();
        self_.browse_model.remove_all();
        self_.browse_known_ids.borrow_mut().clear();
        self_.browse_cursor.replace(None);
        self_.load_more_button.set_visible(false);
    }

    pub fn add_to_library(&self, listing_uid: &str) {
        if let Some(window) = self.main_window() {
            let win_ = window.imp();
            let eg = win_.model.borrow().epic_games.borrow().clone();
            let sender = win_.model.borrow().sender.clone();
            let uid = listing_uid.to_string();

            self.imp().image_load_pool.execute(move || {
                if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                match crate::RUNTIME.block_on(eg.fab_add_to_library(&uid)) {
                    Ok(()) => {
                        debug!("Successfully added {} to library", uid);
                        let _ =
                            sender.send_blocking(crate::ui::messages::Msg::FabAddedToLibrary(uid));
                    }
                    Err(e) => {
                        error!("Failed to add {} to library: {}", uid, e);
                    }
                }
            });
        }
    }

    pub fn on_added_to_library(&self, uid: &str) {
        let self_ = self.imp();
        for i in 0..self_.browse_model.n_items() {
            if let Some(item) = self_.browse_model.item(i) {
                if let Some(data) = item.downcast_ref::<crate::models::fab_data::FabData>() {
                    if let Some(asset) = data.imp().asset.borrow().as_ref() {
                        if asset.asset_id == uid {
                            data.set_property("price-label", "Added ✓");
                            return;
                        }
                    }
                }
            }
        }
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
        if *self_.browse_mode.borrow() {
            self_.browse_model.remove_all();
        }
        self.update_count();
    }

    fn handle_tile_clicked(
        &self,
        asset_widget: &crate::ui::widgets::logged_in::library::asset::EpicAsset,
    ) {
        let self_ = self.imp();
        if let Some(fab_data) = asset_widget.imp().fab_data.borrow().as_ref() {
            if let Some(asset) = fab_data.imp().asset.borrow().as_ref() {
                if *self_.browse_mode.borrow() {
                    self.fetch_listing_detail(&asset.asset_id);
                } else if let Some(details) = self_.details.get() {
                    details.set_fab_asset(asset);
                }
            }
        }
    }

    fn fetch_listing_detail(&self, uid: &str) {
        if let Some(window) = self.main_window() {
            let win_ = window.imp();
            let eg = win_.model.borrow().epic_games.borrow().clone();
            let sender = win_.model.borrow().sender.clone();
            let uid = uid.to_string();

            self.imp().image_load_pool.execute(move || {
                if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }

                let detail = crate::RUNTIME.block_on(eg.fab_listing(&uid));
                let formats = crate::RUNTIME
                    .block_on(eg.fab_listing_ue_formats(&uid))
                    .unwrap_or_default();
                let owned = crate::RUNTIME
                    .block_on(eg.fab_listing_state(&uid))
                    .and_then(|s| s.acquired)
                    .unwrap_or(false);

                if let Some(detail) = detail {
                    let _ = sender.send_blocking(
                        crate::ui::messages::Msg::ProcessFabListingDetail(detail, formats, owned),
                    );
                }
            });
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

    fn handle_add_to_library(
        &self,
        asset_widget: &crate::ui::widgets::logged_in::library::asset::EpicAsset,
    ) {
        if let Some(fab_data) = asset_widget.imp().fab_data.borrow().as_ref() {
            if let Some(asset) = fab_data.imp().asset.borrow().as_ref() {
                self.add_to_library(&asset.asset_id);
            }
        }
    }

    pub fn run_refresh(&self) {
        self.clear();
        self.fetch_fab_assets();
    }
}
