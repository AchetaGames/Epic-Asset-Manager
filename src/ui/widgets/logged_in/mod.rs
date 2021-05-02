use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{glib, CompositeTemplate};

pub(crate) mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/logged_in.ui")]
    pub struct EpicLoggedInBox {}

    #[glib::object_subclass]
    impl ObjectSubclass for EpicLoggedInBox {
        const NAME: &'static str = "EpicLoggedInBox";
        type Type = super::EpicLoggedInBox;
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

    impl ObjectImpl for EpicLoggedInBox {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }

    impl WidgetImpl for EpicLoggedInBox {}
    impl BoxImpl for EpicLoggedInBox {}
}

glib::wrapper! {
    pub struct EpicLoggedInBox(ObjectSubclass<imp::EpicLoggedInBox>)
        @extends gtk::Widget, gtk::Box;
}

impl EpicLoggedInBox {
    pub fn new() -> Self {
        let stack: Self = glib::Object::new(&[]).expect("Failed to create EpicLoggedInBox");

        stack
    }
}
