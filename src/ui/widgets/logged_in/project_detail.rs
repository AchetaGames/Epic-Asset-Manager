use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;
use std::path::PathBuf;

pub(crate) mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::glib::ParamSpec;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/project_detail.ui")]
    pub struct UnrealProjectDetails {
        pub expanded: RefCell<bool>,
        #[template_child]
        pub detail_slider: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub details: TemplateChild<gtk4::Box>,
        #[template_child]
        pub details_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub title: TemplateChild<gtk4::Label>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub actions: gio::SimpleActionGroup,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnrealProjectDetails {
        const NAME: &'static str = "UnrealProjectDetails";
        type Type = super::UnrealProjectDetails;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                expanded: RefCell::new(false),
                detail_slider: TemplateChild::default(),
                details: TemplateChild::default(),
                details_box: TemplateChild::default(),
                title: TemplateChild::default(),
                window: OnceCell::new(),
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

    impl ObjectImpl for UnrealProjectDetails {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpec::new_boolean(
                    "expanded",
                    "expanded",
                    "Is expanded",
                    false,
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
                "expanded" => {
                    let expanded = value.get().unwrap();
                    self.expanded.replace(expanded);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "expanded" => self.expanded.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for UnrealProjectDetails {}
    impl BoxImpl for UnrealProjectDetails {}
}

glib::wrapper! {
    pub struct UnrealProjectDetails(ObjectSubclass<imp::UnrealProjectDetails>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for UnrealProjectDetails {
    fn default() -> Self {
        Self::new()
    }
}

impl UnrealProjectDetails {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn setup_actions(&self) {
        let self_: &imp::UnrealProjectDetails = imp::UnrealProjectDetails::from_instance(self);
        let actions = &self_.actions;
        self.insert_action_group("project_details", Some(actions));

        action!(
            actions,
            "close",
            clone!(@weak self as details => move |_, _| {
                details.set_property("expanded", false).unwrap();
            })
        );
    }

    pub fn set_project(&self, project: crate::models::project_data::Uproject, path: String) {
        let self_: &imp::UnrealProjectDetails = imp::UnrealProjectDetails::from_instance(self);
        let pathbuf = PathBuf::from(path);
        self_.title.set_markup(&format!(
            "<b><u><big>{}</big></u></b>",
            pathbuf.file_stem().unwrap().to_str().unwrap().to_string()
        ));

        if !self.is_expanded() {
            self.set_property("expanded", true).unwrap();
        }
    }

    pub fn is_expanded(&self) -> bool {
        if let Ok(value) = self.property("expanded") {
            if let Ok(id_opt) = value.get::<bool>() {
                return id_opt;
            }
        };
        false
    }
}
