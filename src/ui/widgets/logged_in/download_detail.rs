use gtk::subclass::prelude::*;
use gtk::{self, gio, prelude::*};
use gtk::{glib, CompositeTemplate};

pub(crate) mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/download_detail.ui")]
    pub struct EpicDownloadDetails {
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        pub actions: gio::SimpleActionGroup,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicDownloadDetails {
        const NAME: &'static str = "EpicDownloadDetails";
        type Type = super::EpicDownloadDetails;
        type ParentType = gtk::Box;

        fn new() -> Self {
            Self {
                asset: RefCell::new(None),
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

    impl ObjectImpl for EpicDownloadDetails {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
        }
    }

    impl WidgetImpl for EpicDownloadDetails {}
    impl BoxImpl for EpicDownloadDetails {}
}

glib::wrapper! {
    pub struct EpicDownloadDetails(ObjectSubclass<imp::EpicDownloadDetails>)
        @extends gtk::Widget, gtk::Box;
}

impl EpicDownloadDetails {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLoggedInBox")
    }

    pub fn setup_actions(&self) {
        let self_: &imp::EpicDownloadDetails = imp::EpicDownloadDetails::from_instance(self);
        let actions = &self_.actions;
        self.insert_action_group("download", Some(actions));
    }

    pub fn set_asset(&self, asset: egs_api::api::types::asset_info::AssetInfo) {
        let self_: &imp::EpicDownloadDetails = imp::EpicDownloadDetails::from_instance(self);
        self_.asset.replace(Some(asset.clone()));
    }
}
