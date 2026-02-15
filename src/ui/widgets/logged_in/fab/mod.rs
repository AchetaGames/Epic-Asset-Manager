use glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use log::{debug, error};
use std::path::PathBuf;
use tokio::runtime::Builder;

pub mod version_dialog;

pub mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::ui::widgets::logged_in::library::asset_detail::EpicAssetDetails;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::gio::ListStore;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
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
        pub count_label: TemplateChild<gtk4::Label>,
        #[template_child]
        pub refresh_progress: TemplateChild<gtk4::ProgressBar>,
        pub grid_model: ListStore,
        pub filter_model: gtk4::FilterListModel,
        pub search: RefCell<Option<String>>,
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
                count_label: TemplateChild::default(),
                refresh_progress: TemplateChild::default(),
                grid_model: ListStore::new::<crate::models::fab_data::FabData>(),
                filter_model: gtk4::FilterListModel::new(
                    None::<gtk4::gio::ListStore>,
                    None::<gtk4::CustomFilter>,
                ),
                search: RefCell::new(None),
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

        let filter = gtk4::CustomFilter::new(move |obj| {
            if let Some(data) = obj.downcast_ref::<crate::models::fab_data::FabData>() {
                if let Some(ref s) = search {
                    let name = data.name().to_lowercase();
                    return name.contains(&s.to_lowercase());
                }
                return true;
            }
            false
        });

        self_.filter_model.set_filter(Some(&filter));
    }

    fn update_count(&self) {
        let self_ = self.imp();
        let count = self_.grid_model.n_items();
        self_.count_label.set_label(&format!("{} assets", count));
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

            debug!("Fetching FAB library for account {}", account_id);

            self_.image_load_pool.execute(move || {
                if let Ok(w) = crate::RUNNING.read() {
                    if !*w {
                        return;
                    }
                }

                let fab_library = Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(eg.fab_library_items(account_id));

                if let Some(library) = fab_library {
                    debug!("Got {} FAB assets", library.results.len());
                    for asset in &library.results {
                        if let Ok(w) = crate::RUNNING.read() {
                            if !*w {
                                return;
                            }
                        }

                        let texture = Self::load_fab_thumbnail(asset, &cache_dir);
                        sender
                            .send_blocking(crate::ui::messages::Msg::ProcessFabAsset(
                                asset.clone(),
                                texture,
                            ))
                            .unwrap();
                    }
                } else {
                    error!("Failed to fetch FAB library");
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

    pub fn run_refresh(&self) {
        self.clear();
        self.fetch_fab_assets();
    }
}
