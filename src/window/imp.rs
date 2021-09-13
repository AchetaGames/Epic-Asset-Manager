use super::*;
use crate::models::Model;
use glib::ParamSpec;

#[derive(CompositeTemplate)]
#[template(resource = "/io/github/achetagames/epic_asset_manager/window.ui")]
pub struct EpicAssetManagerWindow {
    #[template_child]
    pub headerbar: TemplateChild<gtk4::HeaderBar>,
    #[template_child]
    pub main_stack: TemplateChild<gtk4::Stack>,
    #[template_child]
    pub logged_in_stack: TemplateChild<crate::ui::widgets::logged_in::EpicLoggedInBox>,
    #[template_child]
    pub sid_box: TemplateChild<crate::ui::widgets::sid_login::SidBox>,
    #[template_child]
    pub progress_message: TemplateChild<gtk4::Label>,
    #[template_child]
    pub download_manager: TemplateChild<crate::ui::widgets::download_manager::EpicDownloadManager>,
    pub model: Model,
}

#[glib::object_subclass]
impl ObjectSubclass for EpicAssetManagerWindow {
    const NAME: &'static str = "EpicAssetManagerWindow";
    type Type = super::EpicAssetManagerWindow;
    type ParentType = gtk4::ApplicationWindow;

    fn new() -> Self {
        Self {
            headerbar: TemplateChild::default(),
            main_stack: TemplateChild::default(),
            logged_in_stack: TemplateChild::default(),
            sid_box: TemplateChild::default(),
            progress_message: TemplateChild::default(),
            download_manager: TemplateChild::default(),
            model: Model::new(),
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

impl ObjectImpl for EpicAssetManagerWindow {
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        // Devel Profile
        if PROFILE == "Devel" {
            obj.style_context().add_class("devel");
        }

        // load latest window state
        obj.load_window_size();
        obj.setup_actions();
        obj.setup_receiver();
    }

    fn properties() -> &'static [ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
            vec![
                ParamSpec::new_string("item", "item", "item", None, glib::ParamFlags::READWRITE),
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

    fn set_property(&self, _obj: &Self::Type, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
        match pspec.name() {
            "item" => {
                let item = value.get::<String>().unwrap();
                self.logged_in_stack.set_property("item", item).unwrap();
            }
            "product" => {
                let product = value.get::<String>().unwrap();
                self.logged_in_stack
                    .set_property("product", product)
                    .unwrap();
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
        match pspec.name() {
            "item" => self
                .logged_in_stack
                .property("item")
                .unwrap_or_else(|_| "".to_value())
                .to_value(),
            "product" => self
                .logged_in_stack
                .property("product")
                .unwrap_or_else(|_| "".to_value())
                .to_value(),
            &_ => unimplemented!(),
        }
    }
}

impl WidgetImpl for EpicAssetManagerWindow {}

impl WindowImpl for EpicAssetManagerWindow {
    // save window state on delete event
    fn close_request(&self, obj: &Self::Type) -> Inhibit {
        if let Err(err) = obj.save_window_size() {
            warn!("Failed to save window state, {}", &err);
        }
        Inhibit(false)
    }
}

impl ApplicationWindowImpl for EpicAssetManagerWindow {}
