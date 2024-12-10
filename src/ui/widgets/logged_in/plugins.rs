use gtk4::{self, glib, subclass::prelude::*, CompositeTemplate};

#[derive(Debug, Clone)]
pub enum Msg {
    AddPlugin(String, String, bool),
}

pub mod imp {
    use super::*;
    use gtk4::gio::ListStore;
    use gtk4::glib::Object;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use threadpool::ThreadPool;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/plugins.ui")]
    pub struct EpicPlugins {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub model: ListStore,
        pub sender: async_channel::Sender<Msg>,
        pub receiver: RefCell<Option<async_channel::Receiver<Msg>>>,
        pub pending: std::sync::RwLock<Vec<Object>>,
        pub load_pool: ThreadPool,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicPlugins {
        const NAME: &'static str = "EpicPlugins";
        type Type = super::EpicPlugins;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            let (sender, receiver) = async_channel::unbounded();
            Self {
                window: OnceCell::new(),
                model: gtk4::gio::ListStore::new::<crate::models::log_data::LogData>(),
                sender,
                receiver: RefCell::new(Some(receiver)),
                pending: std::sync::RwLock::default(),
                load_pool: ThreadPool::with_name("Logs Load Pool".to_string(), 1),
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

    impl ObjectImpl for EpicPlugins {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for EpicPlugins {}
    impl BoxImpl for EpicPlugins {}
}

glib::wrapper! {
    pub struct EpicPlugins(ObjectSubclass<imp::EpicPlugins>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicPlugins {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicPlugins {
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
    }
}
