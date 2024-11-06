use gtk4::glib::clone;
use gtk4::{self, glib, prelude::*, subclass::prelude::*, CompositeTemplate, CustomSorter};
use std::cmp::Ordering;
use std::path::Path;
use std::{iter::Peekable, str::Chars};

pub struct IterPair<'a> {
    pub fst: Peekable<Chars<'a>>,
    pub lst: Peekable<Chars<'a>>,
}

impl<'a> IterPair<'a> {
    pub fn from(i1: Chars<'a>, i2: Chars<'a>) -> Self {
        Self {
            fst: i1.peekable(),
            lst: i2.peekable(),
        }
    }

    pub fn next(&mut self) -> [Option<char>; 2] {
        [self.fst.next(), self.lst.next()]
    }

    pub fn peek(&mut self) -> [Option<&char>; 2] {
        [self.fst.peek(), self.lst.peek()]
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    AddLog(String, String, bool),
}

pub mod imp {
    use super::*;
    use gtk4::gio::ListStore;
    use gtk4::glib::Object;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use threadpool::ThreadPool;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/logs.ui")]
    pub struct EpicLogs {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        #[template_child]
        pub logs: TemplateChild<gtk4::ListView>,
        pub model: ListStore,
        pub sender: async_channel::Sender<Msg>,
        pub receiver: RefCell<Option<async_channel::Receiver<Msg>>>,
        pub pending: std::sync::RwLock<Vec<Object>>,
        pub load_pool: ThreadPool,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicLogs {
        const NAME: &'static str = "EpicLogs";
        type Type = super::EpicLogs;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            let (sender, receiver) = async_channel::unbounded();
            Self {
                window: OnceCell::new(),
                logs: TemplateChild::default(),
                model: ListStore::new::<crate::models::log_data::LogData>(),
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

    impl ObjectImpl for EpicLogs {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_messaging();
        }
    }

    impl WidgetImpl for EpicLogs {}
    impl BoxImpl for EpicLogs {}
}

glib::wrapper! {
    pub struct EpicLogs(ObjectSubclass<imp::EpicLogs>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicLogs {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicLogs {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn setup_messaging(&self) {
        glib::MainContext::default().spawn_local(clone!(
            #[weak(rename_to=logs)]
            self,
            async move {
                let self_ = logs.imp();
                let receiver = self_.receiver.borrow_mut().take().unwrap();
                while let Ok(msg) = receiver.recv().await {
                    logs.update(msg);
                }
            }
        ));
    }

    pub fn update(&self, msg: Msg) {
        let self_ = self.imp();
        match msg {
            Msg::AddLog(path, name, crash) => {
                let line = crate::models::log_data::LogData::new(&path, &name, crash);
                line.set_property("path", &path);
                line.set_property("crash", crash);
                line.set_property("name", &name);
                if let Ok(mut vec) = self_.pending.write() {
                    vec.push(line.upcast());
                }
            }
        };
    }

    pub fn flush_logs(&self) -> bool {
        let self_ = self.imp();
        if let Ok(mut vec) = self_.pending.write() {
            if vec.is_empty() {
                return true;
            }
            self_.model.splice(0, 0, vec.as_slice());
            vec.clear();
        }
        self_.load_pool.active_count() + self_.load_pool.queued_count() != 0
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        let factory = gtk4::SignalListItemFactory::new();
        // Create the children
        factory.connect_setup(move |_factory, item| {
            let row = crate::ui::widgets::logged_in::log_line::EpicLogLine::new();
            let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
            item.set_child(Some(&row));
        });

        // Populate children
        factory.connect_bind(move |_, list_item| {
            let item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            Self::populate_model(item);
        });

        let sorter_model = gtk4::SortListModel::builder()
            .model(&self_.model)
            .sorter(&Self::sorter())
            .build();
        let selection_model = gtk4::NoSelection::new(Some(sorter_model));
        self_.logs.set_model(Some(&selection_model));
        self_.logs.set_factory(Some(&factory));
    }

    fn sorter() -> CustomSorter {
        CustomSorter::new(move |obj1, obj2| {
            let info1 = obj1
                .downcast_ref::<crate::models::log_data::LogData>()
                .unwrap()
                .name();
            let info2 = obj2
                .downcast_ref::<crate::models::log_data::LogData>()
                .unwrap()
                .name();
            if info1.is_none() {
                return gtk4::Ordering::Smaller;
            } else if info2.is_none() {
                return gtk4::Ordering::Larger;
            }

            Self::compare_chars_iters(info2.unwrap().chars(), info1.unwrap().chars())
                .unwrap_or(Ordering::Equal)
                .into()
        })
    }

    fn compare_chars_iters<'a>(c1: Chars<'a>, c2: Chars<'a>) -> Result<Ordering, ()> {
        let mut iters = IterPair::from(c1, c2);

        while let [Some(x), Some(y)] = iters.peek() {
            if x == y {
                iters.next();
            } else if x.is_numeric() && y.is_numeric() {
                match Self::take_numeric(&mut iters.fst).cmp(&Self::take_numeric(&mut iters.lst)) {
                    Ordering::Equal => iters.next(),
                    ref a => return Ok(*a),
                };
            } else {
                return Ok(x.cmp(y));
            }
        }

        Err(())
    }

    fn take_numeric(iter: &mut Peekable<Chars>) -> u32 {
        let mut sum = 0;

        while let Some(p) = iter.peek() {
            match p.to_string().parse::<u32>() {
                Ok(n) => {
                    sum = sum * 10 + n;
                    iter.next();
                }
                _ => break,
            }
        }

        sum
    }

    fn populate_model(list_item: &gtk4::ListItem) {
        let data = list_item
            .item()
            .unwrap()
            .downcast::<crate::models::log_data::LogData>()
            .unwrap();

        let child = list_item
            .child()
            .unwrap()
            .downcast::<crate::ui::widgets::logged_in::log_line::EpicLogLine>()
            .unwrap();
        child.set_property("path", data.path());
        child.set_property("label", data.name());
        child.set_property("crash", data.crash());
    }

    pub fn add_path(&self, path: &str) {
        let self_ = self.imp();
        let location = std::path::PathBuf::from(path);
        let mut project = location.clone();
        project.push("Saved");
        project.push("Logs");
        let s = self_.sender.clone();
        if project.exists() {
            self_.load_pool.execute(move || {
                Self::read_logs_in_path(project.as_path(), false, &s);
            });
        }
        let mut project = location;
        project.push("Saved");
        project.push("Crashes");
        let s = self_.sender.clone();
        if project.exists() {
            self_.load_pool.execute(move || {
                if let Ok(rd) = project.read_dir() {
                    for d in rd.flatten() {
                        if let Ok(w) = crate::RUNNING.read() {
                            if !*w {
                                return;
                            }
                        };
                        let p = d.path();
                        if p.is_dir() {
                            Self::read_logs_in_path(p.as_path(), true, &s.clone());
                        }
                    }
                }
            });
        }
        glib::idle_add_local(clone!(
            #[weak(rename_to=logs)]
            self,
            #[upgrade_or_panic]
            move || {
                if logs.flush_logs() {
                    glib::ControlFlow::Continue
                } else {
                    glib::ControlFlow::Break
                }
            }
        ));
    }

    fn read_logs_in_path(project: &Path, crash: bool, sender: &async_channel::Sender<Msg>) {
        if let Ok(rd) = project.read_dir() {
            for d in rd.flatten() {
                if let Ok(w) = crate::RUNNING.read() {
                    if !*w {
                        return;
                    }
                };
                let p = d.path();
                if p.is_file() {
                    match p.extension() {
                        None => {
                            continue;
                        }
                        Some(ext) => {
                            if ext.to_ascii_lowercase().ne("log") {
                                continue;
                            };
                        }
                    };
                    let metadata = std::fs::metadata(p.as_path()).expect("unable to read metadata");
                    sender
                        .send_blocking(Msg::AddLog(
                            p.to_str().unwrap_or_default().to_string(),
                            metadata.modified().map_or_else(
                                |_| p.to_str().unwrap_or_default().to_string(),
                                |time| {
                                    let t: chrono::DateTime<chrono::Utc> = time.into();
                                    format!("{}", t.format("%Y-%m-%d %T"))
                                },
                            ),
                            crash,
                        ))
                        .unwrap();
                }
            }
        }
    }

    pub fn clear(&self) {
        let self_ = self.imp();
        self_.model.remove_all();
    }
}
