use super::*;
use crate::models::Model;

#[derive(CompositeTemplate)]
#[template(resource = "/io/github/achetagames/epic_asset_manager/window.ui")]
pub struct EpicAssetManagerWindow {
    #[template_child]
    pub headerbar: TemplateChild<gtk::HeaderBar>,
    #[template_child]
    pub main_stack: TemplateChild<gtk::Stack>,
    #[template_child]
    pub logged_in_stack: TemplateChild<crate::ui::widgets::logged_in::EpicLoggedInBox>,
    #[template_child]
    pub sid_box: TemplateChild<crate::ui::widgets::sid_login::SidBox>,
    #[template_child]
    pub progress_message: TemplateChild<gtk::Label>,
    pub settings: gio::Settings,
    pub model: Model,
}

#[glib::object_subclass]
impl ObjectSubclass for EpicAssetManagerWindow {
    const NAME: &'static str = "EpicAssetManagerWindow";
    type Type = super::EpicAssetManagerWindow;
    type ParentType = gtk::ApplicationWindow;

    fn new() -> Self {
        let win = Self {
            headerbar: TemplateChild::default(),
            main_stack: TemplateChild::default(),
            logged_in_stack: TemplateChild::default(),
            sid_box: TemplateChild::default(),
            progress_message: TemplateChild::default(),
            settings: gio::Settings::new(crate::config::APP_ID),
            model: Model::new(),
        };
        win
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
