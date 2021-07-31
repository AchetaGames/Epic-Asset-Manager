use crate::models::category_data::CategoryData;
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*, Label};
use gtk::{gio, glib, CompositeTemplate};
use gtk_macros::action;
use log::{debug, error};
use tokio::runtime::Runtime;

pub(crate) mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use adw::subclass::action_row::ActionRowImpl;
    use gtk::gdk_pixbuf::Pixbuf;
    use gtk::gio;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use threadpool::ThreadPool;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/download_item.ui")]
    pub struct EpicDownloadItem {
        pub actions: gio::SimpleActionGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
        status: RefCell<Option<String>>,
        label: RefCell<Option<String>>,
        thumbnail: RefCell<Option<Pixbuf>>,
        #[template_child]
        pub image: TemplateChild<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicDownloadItem {
        const NAME: &'static str = "EpicDownloadItem";
        type Type = super::EpicDownloadItem;
        type ParentType = adw::ActionRow;

        fn new() -> Self {
            Self {
                actions: gio::SimpleActionGroup::new(),
                window: OnceCell::new(),
                status: RefCell::new(None),
                label: RefCell::new(None),
                thumbnail: RefCell::new(None),
                image: TemplateChild::default(),
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

    impl ObjectImpl for EpicDownloadItem {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
            obj.setup_messaging();
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpec::new_string(
                        "label",
                        "Label",
                        "Label",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_string(
                        "status",
                        "status",
                        "status",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_object(
                        "thumbnail",
                        "Thumbnail",
                        "Thumbnail",
                        Pixbuf::static_type(),
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "label" => {
                    let label = match value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                    {
                        Some(l) => Some(format!("<b><u>{}</u></b>", l)),
                        None => None,
                    };

                    self.label.replace(label);
                }
                "status" => {
                    let status = match value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                    {
                        Some(l) => Some(format!("<i>{}</i>", l)),
                        None => None,
                    };
                    self.status.replace(status);
                }
                "thumbnail" => {
                    let thumbnail: Option<Pixbuf> = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");

                    self.image.set_from_pixbuf(thumbnail.as_ref());
                    self.thumbnail.replace(thumbnail);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "label" => self.label.borrow().to_value(),
                "status" => self.status.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicDownloadItem {}
    impl ActionRowImpl for EpicDownloadItem {}
    impl ListBoxRowImpl for EpicDownloadItem {}
}

glib::wrapper! {
    pub struct EpicDownloadItem(ObjectSubclass<imp::EpicDownloadItem>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::ActionRow, adw::PreferencesRow;
}

impl EpicDownloadItem {
    pub fn new() -> Self {
        let stack: Self = glib::Object::new(&[]).expect("Failed to create EpicDownloadItem");

        stack
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicDownloadItem = imp::EpicDownloadItem::from_instance(self);
        // Do not run this twice
        if let Some(_) = self_.window.get() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    pub fn setup_actions(&self) {
        let self_: &imp::EpicDownloadItem = imp::EpicDownloadItem::from_instance(self);

        self.insert_action_group("download_item", Some(&self_.actions));
    }

    pub fn setup_messaging(&self) {
        let self_: &imp::EpicDownloadItem = imp::EpicDownloadItem::from_instance(self);
    }
}
