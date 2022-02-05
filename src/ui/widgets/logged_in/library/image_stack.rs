use gtk4::glib::{clone, MainContext, Receiver, Sender, PRIORITY_DEFAULT};
use gtk4::subclass::prelude::*;
use gtk4::{self, gdk_pixbuf, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use log::debug;
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

pub(crate) mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use gtk4::gio;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use threadpool::ThreadPool;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/image_stack.ui")]
    pub struct EpicImageOverlay {
        pub image_load_pool: ThreadPool,
        #[template_child]
        pub stack: TemplateChild<adw::Carousel>,
        pub settings: gio::Settings,
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub sender: Sender<super::ImageMsg>,
        pub receiver: RefCell<Option<Receiver<super::ImageMsg>>>,
        asset: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicImageOverlay {
        const NAME: &'static str = "EpicImageOverlay";
        type Type = super::EpicImageOverlay;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
            Self {
                image_load_pool: ThreadPool::with_name("Image Load Pool".to_string(), 5),
                stack: TemplateChild::default(),
                settings: gio::Settings::new(crate::config::APP_ID),
                actions: gio::SimpleActionGroup::new(),
                download_manager: OnceCell::new(),
                sender,
                receiver: RefCell::new(Some(receiver)),
                asset: RefCell::new(None),
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

    impl ObjectImpl for EpicImageOverlay {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
            obj.setup_receiver();
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecString::new(
                    "asset",
                    "Asset",
                    "Asset",
                    None, // Default value
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
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "asset" => {
                    let asset = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.asset.replace(asset);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "asset" => self.asset.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicImageOverlay {}
    impl BoxImpl for EpicImageOverlay {}
}

glib::wrapper! {
    pub struct EpicImageOverlay(ObjectSubclass<imp::EpicImageOverlay>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicImageOverlay {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum ImageMsg {
    DownloadImage(String, egs_api::api::types::asset_info::KeyImage),
    LoadImage(String, egs_api::api::types::asset_info::KeyImage),
    ImageLoaded(Vec<u8>),
}

impl EpicImageOverlay {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn clear(&self) {
        let self_ = self.imp();
        while self_.stack.n_pages() > 0 {
            self_.stack.remove(&self_.stack.nth_page(0));
        }
        self.check_actions();
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

        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn setup_receiver(&self) {
        let self_ = self.imp();
        self_.receiver.borrow_mut().take().unwrap().attach(
            None,
            clone!(@weak self as img => @default-panic, move |msg| {
                img.update(msg);
                glib::Continue(true)
            }),
        );
    }

    pub fn update(&self, msg: ImageMsg) {
        let self_ = self.imp();
        match msg {
            ImageMsg::DownloadImage(asset, image) => {
                if let Some(dm) = self_.download_manager.get() {
                    dm.download_image(image, asset, self_.sender.clone());
                }
            }
            ImageMsg::LoadImage(asset, img) => {
                debug!("Image downloaded");
                if asset.eq(&self.asset()) {
                    debug!("Adding image");
                    self.add_image(&img);
                }
            }
            ImageMsg::ImageLoaded(img) => {
                debug!("Adding image to stack");
                let pixbuf_loader = gdk_pixbuf::PixbufLoader::new();
                pixbuf_loader.write(img.as_slice()).unwrap();
                pixbuf_loader.close().ok();
                let image = gtk4::Picture::for_pixbuf(&pixbuf_loader.pixbuf().unwrap());
                image.set_hexpand(true);
                image.set_vexpand(true);
                self_.stack.append(&image);
                self.check_actions();
            }
        }
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();

        let actions = &self_.actions;
        self.insert_action_group("image_stack", Some(actions));

        self_.stack.connect_page_changed(
            clone!(@weak self as image_stack => move |_, _| image_stack.check_actions();),
        );
        action!(
            actions,
            "next",
            clone!(@weak self as image_stack => move |_, _| {
                image_stack.next();
            })
        );

        action!(
            actions,
            "prev",
            clone!(@weak self as image_stack => move |_, _| {
                image_stack.prev();
            })
        );
    }

    fn next(&self) {
        let self_ = self.imp();
        let image = self_
            .stack
            .nth_page((self_.stack.position().round() as u32) + 1);
        self_.stack.scroll_to(&image, true);
    }

    fn prev(&self) {
        let self_ = self.imp();
        let image = self_
            .stack
            .nth_page((self_.stack.position().round() as u32).saturating_sub(1));
        self_.stack.scroll_to(&image, true);
    }

    pub fn check_actions(&self) {
        let self_ = self.imp();
        get_action!(self_.actions, @prev).set_enabled(
            match self_.stack.position().partial_cmp(&1.0) {
                None | Some(Ordering::Less) => false,
                _ => self_.stack.first_child().is_some(),
            },
        );

        get_action!(self_.actions, @next).set_enabled(
            !matches!(
                self_
                    .stack
                    .position()
                    .partial_cmp(&(self_.stack.n_pages().saturating_sub(2) as f64)),
                None | Some(std::cmp::Ordering::Greater)
            ) && (self_.stack.n_pages() > 0),
        );
    }

    pub fn asset(&self) -> String {
        self.property("asset")
    }

    pub fn add_image(&self, image: &egs_api::api::types::asset_info::KeyImage) {
        debug!("Adding image: {}", image.url);
        let self_ = self.imp();
        let cache_dir = self_.settings.string("cache-directory").to_string();
        let mut cache_path = PathBuf::from(cache_dir);
        cache_path.push("images");
        let name = Path::new(image.url.path())
            .extension()
            .and_then(OsStr::to_str);
        cache_path.push(format!("{}.{}", image.md5, name.unwrap_or(".png")));
        // TODO Have just one sender&receiver per the widget
        let sender = self_.sender.clone();

        let asset = self.asset();
        let img = image.clone();

        self_.image_load_pool.execute(move || {
            if let Ok(mut f) = File::open(cache_path.as_path()) {
                fs::create_dir_all(&cache_path.parent().unwrap()).unwrap();
                let metadata =
                    fs::metadata(&cache_path.as_path()).expect("unable to read metadata");
                let mut buffer = vec![0; metadata.len() as usize];
                f.read_exact(&mut buffer).expect("buffer overflow");
                let pixbuf_loader = gdk_pixbuf::PixbufLoader::new();
                pixbuf_loader.write(&buffer).unwrap();
                pixbuf_loader.close().ok();
                match pixbuf_loader.pixbuf() {
                    None => {}
                    Some(pb) => sender
                        .send(ImageMsg::ImageLoaded(
                            pb.save_to_bufferv("png", &[]).unwrap(),
                        ))
                        .unwrap(),
                };
            } else {
                debug!("Need to download image");
                sender
                    .send(ImageMsg::DownloadImage(asset, img.clone()))
                    .unwrap();
            };
        });
    }
}
