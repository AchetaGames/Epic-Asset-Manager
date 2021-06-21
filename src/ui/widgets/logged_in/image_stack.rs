use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{glib, CompositeTemplate};

pub(crate) mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/image_stack.ui")]
    pub struct EpicImageOverlay {}

    #[glib::object_subclass]
    impl ObjectSubclass for EpicImageOverlay {
        const NAME: &'static str = "EpicImageOverlay";
        type Type = super::EpicImageOverlay;
        type ParentType = gtk::Box;

        fn new() -> Self {
            Self {}
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EpicImageOverlay {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }

    impl WidgetImpl for EpicImageOverlay {}
    impl BoxImpl for EpicImageOverlay {}
}

glib::wrapper! {
    pub struct EpicImageOverlay(ObjectSubclass<imp::EpicImageOverlay>)
        @extends gtk::Widget, gtk::Box;
}

impl EpicImageOverlay {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLoggedInBox")
    }
}
