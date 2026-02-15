use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::gio::ListStore;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/games.ui")]
    pub struct EpicGamesBox {
        pub window: OnceCell<EpicAssetManagerWindow>,
        #[template_child]
        pub games_grid: TemplateChild<gtk4::GridView>,
        #[template_child]
        pub games_search: TemplateChild<gtk4::SearchEntry>,
        pub grid_model: ListStore,
        pub filter_model: gtk4::FilterListModel,
        pub search: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicGamesBox {
        const NAME: &'static str = "EpicGamesBox";
        type Type = super::EpicGamesBox;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
                games_grid: TemplateChild::default(),
                games_search: TemplateChild::default(),
                grid_model: ListStore::new::<crate::models::asset_data::AssetData>(),
                filter_model: gtk4::FilterListModel::new(
                    None::<gtk4::gio::ListStore>,
                    None::<gtk4::CustomFilter>,
                ),
                search: RefCell::new(None),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EpicGamesBox {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_widgets();
        }
    }

    impl WidgetImpl for EpicGamesBox {}
    impl BoxImpl for EpicGamesBox {}
}

glib::wrapper! {
    pub struct EpicGamesBox(ObjectSubclass<imp::EpicGamesBox>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl Default for EpicGamesBox {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicGamesBox {
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
    }

    fn setup_widgets(&self) {
        let self_ = self.imp();

        // Connect search
        self_.games_search.connect_search_changed(clone!(
            #[weak(rename_to=games)]
            self,
            move |entry| {
                let text = entry.text();
                let search = if text.is_empty() {
                    None
                } else {
                    Some(text.to_string())
                };
                games.imp().search.replace(search);
                games.update_filter();
            }
        ));
    }

    fn setup_grid(&self) {
        let self_ = self.imp();

        let factory = gtk4::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let row = crate::ui::widgets::logged_in::library::asset::EpicAsset::new();
            let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
            item.set_child(Some(&row));
        });

        factory.connect_bind(move |_, list_item| {
            let item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            if let Some(data) = item.item() {
                if let Some(asset_data) = data.downcast_ref::<crate::models::asset_data::AssetData>() {
                    if let Some(child) = item.child() {
                        if let Some(asset) = child.downcast_ref::<crate::ui::widgets::logged_in::library::asset::EpicAsset>() {
                            asset.set_data(asset_data);
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

        self_.games_grid.set_model(Some(&selection_model));
        self_.games_grid.set_factory(Some(&factory));
    }

    fn update_filter(&self) {
        let self_ = self.imp();
        let search = self_.search.borrow().clone();

        let filter = gtk4::CustomFilter::new(move |obj| {
            if let Some(data) = obj.downcast_ref::<crate::models::asset_data::AssetData>() {
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

    pub fn add_game(&self, asset_data: &crate::models::asset_data::AssetData) {
        let self_ = self.imp();
        self_.grid_model.append(asset_data);
    }

    pub fn add_asset_info(
        &self,
        asset: &egs_api::api::types::asset_info::AssetInfo,
        image: Option<gtk4::gdk::Texture>,
    ) {
        let data = crate::models::asset_data::AssetData::new(asset, image);
        self.add_game(&data);
    }

    pub fn clear_games(&self) {
        let self_ = self.imp();
        self_.grid_model.remove_all();
    }
}
