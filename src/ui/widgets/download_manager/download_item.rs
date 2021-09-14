use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;
use log::warn;

pub(crate) mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use adw::subclass::action_row::ActionRowImpl;
    use gtk4::gdk_pixbuf::Pixbuf;
    use gtk4::gio;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/download_item.ui")]
    pub struct EpicDownloadItem {
        pub actions: gio::SimpleActionGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
        status: RefCell<Option<String>>,
        label: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        pub total_size: RefCell<u128>,
        pub downloaded_size: RefCell<u128>,
        pub total_files: RefCell<u64>,
        pub extracted_files: RefCell<u64>,
        thumbnail: RefCell<Option<Pixbuf>>,
        #[template_child]
        pub image: TemplateChild<gtk4::Image>,
        #[template_child]
        pub stack: TemplateChild<gtk4::Stack>,
        #[template_child]
        pub download_progress: TemplateChild<gtk4::ProgressBar>,
        #[template_child]
        pub extraction_progress: TemplateChild<gtk4::ProgressBar>,
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
                path: RefCell::new(None),
                total_size: RefCell::new(0),
                downloaded_size: RefCell::new(0),
                total_files: RefCell::new(0),
                extracted_files: RefCell::new(0),
                thumbnail: RefCell::new(None),
                image: TemplateChild::default(),
                stack: TemplateChild::default(),
                download_progress: TemplateChild::default(),
                extraction_progress: TemplateChild::default(),
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

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder(
                        "finished",
                        &[],
                        <()>::static_type().into(),
                    )
                    .flags(glib::SignalFlags::ACTION)
                    .build()]
                });
            SIGNALS.as_ref()
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
                        "path",
                        "Path",
                        "Path",
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
                    let label = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("<b><u>{}</u></b>", l));

                    self.label.replace(label);
                }
                "path" => {
                    let init = self.path.borrow().is_none();
                    let path = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`");

                    self.path.replace(path.clone());
                    if init {
                        if let Some(p) = path {
                            action!(self.actions, "open", move |_, _| {
                                if gtk4::gio::AppInfo::launch_default_for_uri(
                                    &format!("file://{}/data", p),
                                    None::<&gtk4::gio::AppLaunchContext>,
                                )
                                .is_err()
                                {
                                    warn!("Unable to open path")
                                }
                            });
                        }
                    }
                }
                "status" => {
                    let status = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("<i>{}</i>", l));
                    self.stack.set_visible_child_name("label");
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
                "path" => self.path.borrow().to_value(),
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
        @extends gtk4::Widget, gtk4::ListBoxRow, adw::ActionRow, adw::PreferencesRow;
}

impl Default for EpicDownloadItem {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicDownloadItem {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicDownloadItem")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicDownloadItem = imp::EpicDownloadItem::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    pub fn setup_actions(&self) {
        let self_: &imp::EpicDownloadItem = imp::EpicDownloadItem::from_instance(self);

        self.insert_action_group("download_item", Some(&self_.actions));
    }

    pub fn setup_messaging(&self) {
        let _self_: &imp::EpicDownloadItem = imp::EpicDownloadItem::from_instance(self);
    }

    pub fn set_total_size(&self, size: u128) {
        let self_: &imp::EpicDownloadItem = imp::EpicDownloadItem::from_instance(self);
        self_.total_size.replace(size);
    }

    pub fn path(&self) -> Option<String> {
        if let Ok(value) = self.property("path") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn set_total_files(&self, count: u64) {
        let self_: &imp::EpicDownloadItem = imp::EpicDownloadItem::from_instance(self);
        self_.total_files.replace(count);
    }

    pub fn file_processed(&self) {
        let self_: &imp::EpicDownloadItem = imp::EpicDownloadItem::from_instance(self);
        self_.stack.set_visible_child_name("progress");
        let new_count = *self_.extracted_files.borrow() + 1;
        let total = *self_.total_files.borrow();
        self_
            .extraction_progress
            .set_fraction(new_count as f64 / total as f64);
        self_.extracted_files.replace(new_count);
        self.parent();
        if new_count == total {
            self.set_property("status", "Finished".to_string()).unwrap();
            glib::timeout_add_seconds_local(
                15,
                clone!(@weak self as obj => @default-panic, move || {
                    obj.emit_by_name("finished", &[]).unwrap();
                    glib::Continue(false)
                }),
            );
        };
    }

    pub fn add_downloaded_size(&self, size: u128) {
        let self_: &imp::EpicDownloadItem = imp::EpicDownloadItem::from_instance(self);
        self_.stack.set_visible_child_name("progress");
        let new_size = *self_.downloaded_size.borrow() + size;
        let total = *self_.total_size.borrow();
        self_
            .download_progress
            .set_fraction(new_size as f64 / total as f64);
        self_.downloaded_size.replace(new_size);
    }

    pub fn progress(&self) -> f64 {
        let self_: &imp::EpicDownloadItem = imp::EpicDownloadItem::from_instance(self);
        let new_size = *self_.downloaded_size.borrow();
        let total = *self_.total_size.borrow();
        let new_count = *self_.extracted_files.borrow();
        let total_count = *self_.total_files.borrow();
        ((if total != 0 {
            new_size as f64 / total as f64
        } else {
            0.0
        }) / 2.0)
            + ((if total_count != 0 {
                new_count as f64 / total_count as f64
            } else {
                0.0
            }) / 2.0)
    }
}
