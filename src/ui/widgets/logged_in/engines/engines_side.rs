use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;

pub mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString, ParamSpecUInt};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/engines_side.ui")]
    pub struct EpicEnginesSide {
        #[template_child]
        pub stack: TemplateChild<gtk4::Stack>,
        #[template_child]
        pub details:
            TemplateChild<crate::ui::widgets::logged_in::engines::engine_detail::EpicEngineDetails>,
        #[template_child]
        pub install:
            TemplateChild<crate::ui::widgets::logged_in::engines::install::EpicEngineInstall>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        selected: RefCell<Option<String>>,
        title: RefCell<Option<String>>,
        position: RefCell<u32>,
        expanded: RefCell<bool>,
        pub actions: gio::SimpleActionGroup,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEnginesSide {
        const NAME: &'static str = "EpicEnginesSide";
        type Type = super::EpicEnginesSide;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                stack: TemplateChild::default(),
                details: TemplateChild::default(),
                install: TemplateChild::default(),
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                selected: RefCell::new(None),
                title: RefCell::new(None),
                position: RefCell::new(0),
                expanded: RefCell::new(false),
                actions: gio::SimpleActionGroup::new(),
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

    impl ObjectImpl for EpicEnginesSide {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_actions();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("expanded").build(),
                    ParamSpecString::builder("selected").build(),
                    ParamSpecString::builder("title").build(),
                    ParamSpecUInt::builder("position")
                        .minimum(0)
                        .default_value(0)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "expanded" => {
                    let expanded = value.get().unwrap();
                    self.expanded.replace(expanded);
                }
                "selected" => {
                    let selected = value.get().unwrap();
                    self.selected.replace(selected);
                    self.details.set_property(pspec.name(), value);
                }
                "title" => {
                    let title = value.get().unwrap();
                    self.title.replace(title);
                }
                "position" => {
                    let position = value.get().unwrap();
                    self.position.replace(position);
                    self.details.set_property(pspec.name(), value);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "expanded" => self.expanded.borrow().to_value(),
                "selected" => self.selected.borrow().to_value(),
                "position" => self.position.borrow().to_value(),
                "title" => self.title.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicEnginesSide {}

    impl BoxImpl for EpicEnginesSide {}
}

glib::wrapper! {
    pub struct EpicEnginesSide(ObjectSubclass<imp::EpicEnginesSide>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicEnginesSide {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicEnginesSide {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        self_.details.set_window(window);
        self_.install.set_window(window);
        self.update_docker();
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.install.set_download_manager(dm);
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("engines_side", Some(actions));

        action!(
            actions,
            "close",
            clone!(
                #[weak(rename_to=side)]
                self,
                move |_, _| {
                    side.collapse();
                }
            )
        );
    }

    pub fn set_data(&self, data: &crate::models::engine_data::EngineData) {
        let self_ = self.imp();
        if let Some(title) = &data.version() {
            self.set_property("title", format!("{title}"));
        }
        self.set_property("visible", true);
        self_.details.set_data(data);
        self_.stack.set_visible_child_name("details");
    }

    pub fn selected(&self) -> Option<String> {
        self.property("selected")
    }

    pub fn position(&self) -> u32 {
        self.property("position")
    }

    pub fn collapse(&self) {
        let self_ = self.imp();
        self.set_property("expanded", false);
        self.set_property("visible", false);
        if let Some(w) = self_.window.get() {
            let w_ = w.imp();
            let l = w_.logged_in_stack.clone();
            let l_ = l.imp();
            let e = l_.engines.imp();
            if let Some(m) = e.engine_grid.model() {
                m.unselect_item(self.position());
            }
        }
    }

    pub fn add_engine(&self) {
        let self_ = self.imp();
        self.set_property("visible", true);
        self.set_property("title", "Install Engine");
        self_.stack.set_visible_child_name("install");
        self_.install.add_engine();
    }

    pub fn path(&self) -> Option<String> {
        let self_ = self.imp();
        // TODO: Check if we are on install tab and return empty
        self_.details.path()
    }

    pub fn update_docker(&self) {
        let self_ = self.imp();
        self_.install.update_docker();
    }
}
