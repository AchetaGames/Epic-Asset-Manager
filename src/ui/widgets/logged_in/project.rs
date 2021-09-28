use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub(crate) mod imp {
    use super::*;
    use gtk4::glib::{ParamSpec, SignalHandlerId};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/project.ui")]
    pub struct EpicProject {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        name: RefCell<Option<String>>,
        pub data: RefCell<Option<crate::models::project_data::ProjectData>>,
        pub handler: RefCell<Option<SignalHandlerId>>,
        #[template_child]
        pub thumbnail: TemplateChild<gtk4::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicProject {
        const NAME: &'static str = "EpicProject";
        type Type = super::EpicProject;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                name: RefCell::new(None),
                data: RefCell::new(None),
                handler: RefCell::new(None),
                thumbnail: TemplateChild::default(),
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

    impl ObjectImpl for EpicProject {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpec::new_string(
                    "name",
                    "Name",
                    "Name",
                    None,
                    glib::ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "name" => {
                    let name = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("<span size=\"xx-large\"><b><u>{}</u></b></span>", l));
                    self.name.replace(name);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicProject {}
    impl BoxImpl for EpicProject {}
}

glib::wrapper! {
    pub struct EpicProject(ObjectSubclass<imp::EpicProject>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicProject {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicProject {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicProject = imp::EpicProject::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_: &imp::EpicProject = imp::EpicProject::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn set_data(&self, data: &crate::models::project_data::ProjectData) {
        let self_: &imp::EpicProject = imp::EpicProject::from_instance(self);
        if let Some(d) = self_.data.take() {
            if let Some(id) = self_.handler.take() {
                d.disconnect(id);
            }
        }
        self_.data.replace(Some(data.clone()));
        self.set_property("name", &data.name()).unwrap();
        if let Some(pix) = data.image() {
            self_.thumbnail.set_from_pixbuf(Some(&pix))
        }

        if let Ok(id) = data.connect_local(
            "finished",
            false,
            clone!(@weak self as project, @weak data => @default-return None, move |_| {
                let self_: &imp::EpicProject = imp::EpicProject::from_instance(&project);
                if let Some(pix) = data.image() {
                    self_.thumbnail.set_from_pixbuf(Some(&pix))
                }
                None
            }),
        ) {
            self_.handler.replace(Some(id));
        }
    }
}
