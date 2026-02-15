use crate::ui::widgets::download_manager::asset::Asset;
use adw::subclass::prelude::AdwWindowImpl;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::window::EpicAssetManagerWindow;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/add_to_project_dialog.ui")]
    pub struct EpicAddToProjectDialog {
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        pub selected_project_path: RefCell<Option<String>>,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub grid_model: gio::ListStore,
        #[template_child]
        pub projects_grid: TemplateChild<gtk4::GridView>,
        #[template_child]
        pub add_button: TemplateChild<gtk4::Button>,
        #[template_child]
        pub overwrite_check: TemplateChild<gtk4::CheckButton>,
        #[template_child]
        pub no_projects_bar: TemplateChild<gtk4::InfoBar>,
        #[template_child]
        pub asset_name_label: TemplateChild<gtk4::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicAddToProjectDialog {
        const NAME: &'static str = "EpicAddToProjectDialog";
        type Type = super::EpicAddToProjectDialog;
        type ParentType = adw::Window;

        fn new() -> Self {
            Self {
                asset: RefCell::new(None),
                selected_project_path: RefCell::new(None),
                download_manager: OnceCell::new(),
                window: OnceCell::new(),
                grid_model: gio::ListStore::new::<crate::models::project_data::ProjectData>(),
                projects_grid: TemplateChild::default(),
                add_button: TemplateChild::default(),
                overwrite_check: TemplateChild::default(),
                no_projects_bar: TemplateChild::default(),
                asset_name_label: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EpicAddToProjectDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_grid();
            self.obj().setup_actions();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder("asset-added")
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for EpicAddToProjectDialog {}
    impl WindowImpl for EpicAddToProjectDialog {}
    impl AdwWindowImpl for EpicAddToProjectDialog {}
}

glib::wrapper! {
    pub struct EpicAddToProjectDialog(ObjectSubclass<imp::EpicAddToProjectDialog>)
        @extends gtk4::Widget, gtk4::Window, adw::Window,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl Default for EpicAddToProjectDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicAddToProjectDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_transient_for(&self, parent: Option<&crate::window::EpicAssetManagerWindow>) {
        gtk4::prelude::GtkWindowExt::set_transient_for(
            self,
            parent.map(|w| w.upcast_ref::<gtk4::Window>()),
        );
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_ = self.imp();
        if self_.download_manager.get().is_none() {
            self_.download_manager.set(dm.clone()).unwrap();
        }
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        if self_.window.get().is_none() {
            self_.window.set(window.clone()).unwrap();
        }
    }

    fn setup_grid(&self) {
        let self_ = self.imp();
        let factory = gtk4::SignalListItemFactory::new();

        factory.connect_setup(clone!(
            #[weak(rename_to=dialog)]
            self,
            move |_factory, item| {
                let item = item.downcast_ref::<gtk4::ListItem>().unwrap();

                // Create a simple project tile
                let tile = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
                tile.set_width_request(160);
                tile.set_height_request(140);
                tile.set_halign(gtk4::Align::Center);
                tile.add_css_class("card");
                tile.set_margin_start(4);
                tile.set_margin_end(4);
                tile.set_margin_top(4);
                tile.set_margin_bottom(4);

                let picture = gtk4::Picture::new();
                picture.set_width_request(150);
                picture.set_height_request(90);
                picture.set_content_fit(gtk4::ContentFit::Cover);
                picture.set_margin_top(4);
                picture.set_margin_start(4);
                picture.set_margin_end(4);

                let label = gtk4::Label::new(None);
                label.set_halign(gtk4::Align::Center);
                label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                label.set_max_width_chars(18);
                label.set_margin_bottom(4);

                tile.append(&picture);
                tile.append(&label);

                // Store references for binding
                item.set_child(Some(&tile));
            }
        ));

        factory.connect_bind(clone!(
            #[weak(rename_to=dialog)]
            self,
            move |_factory, item| {
                let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
                if let Some(project_data) = item
                    .item()
                    .and_downcast::<crate::models::project_data::ProjectData>()
                {
                    if let Some(tile) = item.child().and_downcast::<gtk4::Box>() {
                        // Get the picture and label from the tile
                        let mut child = tile.first_child();
                        while let Some(widget) = child {
                            if let Some(picture) = widget.downcast_ref::<gtk4::Picture>() {
                                if let Some(thumb) = project_data.image() {
                                    picture.set_paintable(Some(&thumb));
                                } else {
                                    // Set default icon
                                    let icon_theme =
                                        gtk4::IconTheme::for_display(&picture.display());
                                    let icon = icon_theme.lookup_icon(
                                        "folder-symbolic",
                                        &[],
                                        90,
                                        1,
                                        gtk4::TextDirection::None,
                                        gtk4::IconLookupFlags::empty(),
                                    );
                                    picture.set_paintable(Some(&icon));
                                }
                            } else if let Some(label) = widget.downcast_ref::<gtk4::Label>() {
                                label.set_label(&project_data.name().unwrap_or_default());
                            }
                            child = widget.next_sibling();
                        }
                    }
                }
            }
        ));

        let selection_model = gtk4::SingleSelection::new(Some(self_.grid_model.clone()));
        selection_model.set_autoselect(false);
        selection_model.set_can_unselect(true);

        selection_model.connect_selection_changed(clone!(
            #[weak(rename_to=dialog)]
            self,
            move |model, _, _| {
                dialog.on_selection_changed(model);
            }
        ));

        self_.projects_grid.set_model(Some(&selection_model));
        self_.projects_grid.set_factory(Some(&factory));
    }

    fn setup_actions(&self) {
        let self_ = self.imp();

        self_.add_button.connect_clicked(clone!(
            #[weak(rename_to=dialog)]
            self,
            move |_| {
                dialog.add_to_project();
            }
        ));
    }

    fn on_selection_changed(&self, model: &gtk4::SingleSelection) {
        let self_ = self.imp();

        if let Some(item) = model.selected_item() {
            if let Some(project_data) =
                item.downcast_ref::<crate::models::project_data::ProjectData>()
            {
                if let Some(path) = project_data.path() {
                    // Get the project directory (parent of .uproject file)
                    let project_path = std::path::Path::new(&path);
                    if let Some(parent) = project_path.parent() {
                        self_
                            .selected_project_path
                            .replace(Some(parent.to_string_lossy().to_string()));
                        self_.add_button.set_sensitive(true);
                        return;
                    }
                }
            }
        }

        self_.selected_project_path.replace(None);
        self_.add_button.set_sensitive(false);
    }

    pub fn set_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_ = self.imp();
        self_.asset.replace(Some(asset.clone()));

        // Update the label
        if let Some(title) = &asset.title {
            self_
                .asset_name_label
                .set_label(&format!("Add \"{}\" to project:", title));
        }
    }

    pub fn load_projects(&self) {
        let self_ = self.imp();
        self_.grid_model.remove_all();

        if let Some(window) = self_.window.get() {
            let w_ = window.imp();
            let logged_in = w_.logged_in_stack.clone();
            let l_ = logged_in.imp();
            let projects_widget = l_.projects.imp();

            // Get all project data from the projects widget's grid_model
            let projects_grid = &projects_widget.grid_model;
            let n_items = projects_grid.n_items();

            if n_items == 0 {
                self_.no_projects_bar.set_visible(true);
            } else {
                self_.no_projects_bar.set_visible(false);
                for i in 0..n_items {
                    if let Some(item) = projects_grid.item(i) {
                        if let Some(project_data) =
                            item.downcast_ref::<crate::models::project_data::ProjectData>()
                        {
                            self_.grid_model.append(project_data);
                        }
                    }
                }
            }
        }
    }

    fn add_to_project(&self) {
        let self_ = self.imp();

        let selected_path = self_.selected_project_path.borrow().clone();
        let Some(project_path) = selected_path else {
            return;
        };

        let Some(dm) = self_.download_manager.get() else {
            return;
        };

        let Some(asset_info) = self_.asset.borrow().clone() else {
            return;
        };

        // Get the first release ID
        let Some(release_info) = asset_info.release_info.as_ref() else {
            return;
        };

        let Some(first_release) = release_info.first() else {
            return;
        };

        let Some(release_id) = &first_release.app_id else {
            return;
        };

        let overwrite = self_.overwrite_check.is_active();

        // Start download with copy action
        dm.add_asset_download(
            release_id.clone(),
            asset_info,
            &None,
            Some(vec![
                crate::ui::widgets::download_manager::PostDownloadAction::Copy(
                    project_path,
                    overwrite,
                ),
            ]),
        );

        self.emit_by_name::<()>("asset-added", &[]);
        self.close();
    }

    pub fn present(&self) {
        self.load_projects();
        gtk4::prelude::GtkWindowExt::present(self);
    }
}
