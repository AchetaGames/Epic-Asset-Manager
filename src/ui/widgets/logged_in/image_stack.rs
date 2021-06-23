use gtk::glib::{clone, MainContext, PRIORITY_DEFAULT};
use gtk::subclass::prelude::*;
use gtk::{self, gdk_pixbuf, prelude::*};
use gtk::{glib, CompositeTemplate};
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

pub(crate) mod imp {
    use super::*;
    use gtk::gio;
    use threadpool::ThreadPool;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/image_stack.ui")]
    pub struct EpicImageOverlay {
        pub image_load_pool: ThreadPool,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        pub settings: gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicImageOverlay {
        const NAME: &'static str = "EpicImageOverlay";
        type Type = super::EpicImageOverlay;
        type ParentType = gtk::Box;

        fn new() -> Self {
            Self {
                image_load_pool: ThreadPool::with_name("Image Load Pool".to_string(), 5),
                stack: TemplateChild::default(),
                settings: gio::Settings::new(crate::config::APP_ID),
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

    pub fn clear(&self) {
        let self_: &imp::EpicImageOverlay = imp::EpicImageOverlay::from_instance(self);
        while let Some(el) = self_.stack.first_child() {
            self_.stack.remove(&el)
        }
    }

    pub fn add_image(&self, image: &egs_api::api::types::asset_info::KeyImage) {
        let self_: &imp::EpicImageOverlay = imp::EpicImageOverlay::from_instance(self);
        let cache_dir = self_.settings.string("cache-directory").to_string().clone();
        let mut cache_path = PathBuf::from(cache_dir);
        cache_path.push("images");
        let name = Path::new(image.url.path())
            .extension()
            .and_then(OsStr::to_str);
        cache_path.push(format!("{}.{}", image.md5, name.unwrap_or(&".png")));
        // TODO Have just one sender&receiver per the widget
        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
        receiver.attach(
            None,
            clone!(@weak self as image_stack => @default-panic, move |img: Vec<u8>| {
                let self_: &imp::EpicImageOverlay = imp::EpicImageOverlay::from_instance(&image_stack);
                let pixbuf_loader = gdk_pixbuf::PixbufLoader::new();
                pixbuf_loader.write(&img.as_slice()).unwrap();
                pixbuf_loader.close().ok();
                let image = gtk::Image::from_pixbuf(pixbuf_loader.pixbuf().as_ref());
                self_.stack.add_child(&image);
                glib::Continue(true)
            }),
        );

        self_.image_load_pool.execute(move || {
            match File::open(cache_path.as_path()) {
                Ok(mut f) => {
                    let metadata =
                        fs::metadata(&cache_path.as_path()).expect("unable to read metadata");
                    let mut buffer = vec![0; metadata.len() as usize];
                    f.read(&mut buffer).expect("buffer overflow");
                    let pixbuf_loader = gdk_pixbuf::PixbufLoader::new();
                    pixbuf_loader.write(&buffer).unwrap();
                    pixbuf_loader.close().ok();
                    match pixbuf_loader.pixbuf() {
                        None => {}
                        Some(pb) => {
                            let width = pb.width();
                            let height = pb.height();
                            let width_percent = 300.0 / width as f64;
                            let height_percent = 500.0 / height as f64;
                            let percent = if height_percent < width_percent {
                                height_percent
                            } else {
                                width_percent
                            };
                            let desired = (width as f64 * percent, height as f64 * percent);
                            sender
                                .send(
                                    pb.scale_simple(
                                        desired.0.round() as i32,
                                        desired.1.round() as i32,
                                        gdk_pixbuf::InterpType::Bilinear,
                                    )
                                    .unwrap()
                                    .save_to_bufferv("png", &[])
                                    .unwrap(),
                                )
                                .unwrap()
                        }
                    };
                }
                Err(_) => {
                    println!("Need to load image");
                }
            };
        })
    }
}
