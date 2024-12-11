use crate::ui::widgets::download_manager::asset::Asset;
use crate::ui::widgets::download_manager::docker::Docker;
use crate::ui::widgets::download_manager::epic_file::EpicFile;
use crate::ui::widgets::download_manager::PostDownloadAction;
use gtk4::glib::clone;
use gtk4::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ItemType")]
pub enum ItemType {
    #[default]
    Unknown,
    Asset,
    Docker,
    Epic,
}

pub mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::gdk::Texture;
    use gtk4::gio;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/download_item.ui")]
    pub struct EpicDownloadItem {
        pub actions: gio::SimpleActionGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_manager: OnceCell<EpicDownloadManager>,
        status: RefCell<Option<String>>,
        label: RefCell<Option<String>>,
        asset: RefCell<Option<String>>,
        version: RefCell<Option<String>>,
        release: RefCell<Option<String>>,
        paused: RefCell<bool>,
        canceled: RefCell<bool>,
        item_type: RefCell<ItemType>,
        speed: RefCell<Option<String>>,
        target: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        pub total_size: RefCell<u128>,
        pub downloaded_size: RefCell<u128>,
        pub total_files: RefCell<u64>,
        pub extracted_files: RefCell<u64>,
        pub post_actions: RefCell<Vec<crate::ui::widgets::download_manager::PostDownloadAction>>,
        pub speed_queue: RefCell<VecDeque<(chrono::DateTime<chrono::Utc>, u128)>>,
        thumbnail: RefCell<Option<Texture>>,
        #[template_child]
        pub pause_button: TemplateChild<gtk4::Button>,
        #[template_child]
        pub download_progress: TemplateChild<gtk4::ProgressBar>,
        #[template_child]
        pub extraction_progress: TemplateChild<gtk4::ProgressBar>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicDownloadItem {
        const NAME: &'static str = "EpicDownloadItem";
        type Type = super::EpicDownloadItem;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                actions: gio::SimpleActionGroup::new(),
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                status: RefCell::new(None),
                label: RefCell::new(None),
                asset: RefCell::new(None),
                version: RefCell::new(None),
                release: RefCell::new(None),
                paused: RefCell::new(false),
                canceled: RefCell::new(false),
                item_type: RefCell::new(ItemType::Unknown),
                speed: RefCell::new(None),
                target: RefCell::new(None),
                path: RefCell::new(None),
                total_size: RefCell::new(0),
                downloaded_size: RefCell::new(0),
                total_files: RefCell::new(0),
                extracted_files: RefCell::new(0),
                post_actions: RefCell::new(vec![]),
                speed_queue: RefCell::new(VecDeque::new()),
                thumbnail: RefCell::new(None),
                pause_button: TemplateChild::default(),
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
        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecEnum::builder::<ItemType>("item-type")
                        .default_value(ItemType::Unknown)
                        .build(),
                    glib::ParamSpecString::builder("label").build(),
                    glib::ParamSpecBoolean::builder("paused").build(),
                    glib::ParamSpecBoolean::builder("canceled").build(),
                    glib::ParamSpecString::builder("asset").build(),
                    glib::ParamSpecString::builder("version").build(),
                    glib::ParamSpecString::builder("release").build(),
                    glib::ParamSpecString::builder("speed").build(),
                    glib::ParamSpecString::builder("target").build(),
                    glib::ParamSpecString::builder("path").build(),
                    glib::ParamSpecString::builder("status").build(),
                    glib::ParamSpecObject::builder::<Texture>("thumbnail").build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder("finished")
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "label" => {
                    let label = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("{l}"));

                    self.label.replace(label);
                }
                "path" => {
                    let init = self.path.borrow().is_none();
                    let path = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`");

                    self.path.replace(path);
                    if init {
                        action!(
                            self.actions,
                            "open",
                            clone!(
                                #[weak(rename_to=imp)]
                                self,
                                move |_, _| {
                                    let obj = imp.obj();
                                    obj.open_path();
                                }
                            )
                        );
                    }
                }
                "status" => {
                    let status = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("{l}"));
                }
                "target" => {
                    let target = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`");
                    self.target.replace(target);
                }
                "paused" => {
                    let paused = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.paused.replace(paused);
                }
                "item-type" => {
                    let item_type = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.item_type.replace(item_type);
                }
                "canceled" => {
                    let canceled = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.canceled.replace(canceled);
                }
                "speed" => {
                    let speed = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`");
                    self.speed.replace(speed);
                }
                "asset" => {
                    let asset = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`");
                    self.asset.replace(asset);
                }
                "version" => {
                    let version = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`");
                    self.version.replace(version);
                }
                "release" => {
                    let release = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`");
                    self.release.replace(release);
                }
                "thumbnail" => {}
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "label" => self.label.borrow().to_value(),
                "target" => self.target.borrow().to_value(),
                "asset" => self.asset.borrow().to_value(),
                "version" => self.version.borrow().to_value(),
                "release" => self.release.borrow().to_value(),
                "status" => self.status.borrow().to_value(),
                "paused" => self.paused.borrow().to_value(),
                "canceled" => self.canceled.borrow().to_value(),
                "item-type" => self.item_type.borrow().to_value(),
                "speed" => self.speed.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            obj.setup_messaging();
            obj.setup_timer();
        }
    }

    impl WidgetImpl for EpicDownloadItem {}
    impl BoxImpl for EpicDownloadItem {}
}

glib::wrapper! {
    pub struct EpicDownloadItem(ObjectSubclass<imp::EpicDownloadItem>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicDownloadItem {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicDownloadItem {
    pub fn new() -> Self {
        glib::Object::new()
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

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    pub fn setup_timer(&self) {
        glib::timeout_add_seconds_local(
            1,
            clone!(
                #[weak(rename_to=obj)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    obj.speed_update();
                    glib::ControlFlow::Continue
                }
            ),
        );
    }

    fn speed_update(&self) {
        let self_ = self.imp();
        if self.canceled() || self.paused() {
            self.set_property("speed", "Paused/Cancelled".to_string());
            return;
        }
        if let Some(speed) = {
            let queue = &mut *self_.speed_queue.borrow_mut();
            if queue.len() <= 1 {
                // self.set_property("speed", "Starting download".to_string());
                return;
            }
            let mut downloaded = 0_u128;
            let start = queue.front().unwrap().0;
            let end = queue.back().unwrap().0;
            let mut pop_counter = 0;
            for (t, s) in &*queue {
                if end - *t > chrono::Duration::seconds(1) {
                    pop_counter += 1;
                }
                downloaded += s;
            }
            for _ in 0..pop_counter {
                queue.pop_front();
            }

            let time = end - start;
            if time > chrono::Duration::seconds(1) {
                Some(downloaded / (time.num_milliseconds().abs() / 1000) as u128)
            } else {
                None
            }
        } {
            let byte = byte_unit::Byte::from_u128(speed)
                .unwrap_or_default()
                .get_appropriate_unit(byte_unit::UnitType::Decimal);
            self.set_property("speed", format!("Downloading - {byte:.1}/s"));
        };
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();

        self.insert_action_group("download_item", Some(&self_.actions));
        action!(
            self_.actions,
            "cancel",
            clone!(
                #[weak(rename_to=item)]
                self,
                move |_, _| {
                    item.cancel();
                }
            )
        );

        get_action!(self_.actions, @cancel).set_enabled(false);

        action!(
            self_.actions,
            "pause",
            clone!(
                #[weak(rename_to=item)]
                self,
                move |_, _| {
                    item.pause();
                }
            )
        );
        get_action!(self_.actions, @pause).set_enabled(false);
    }

    fn cancel(&self) {
        let self_ = self.imp();
        get_action!(self_.actions, @cancel).set_enabled(false);
        get_action!(self_.actions, @pause).set_enabled(false);
        self.set_property("canceled", true);

        if let Some(dm) = self_.download_manager.get() {
            match self.item_type() {
                ItemType::Unknown => {}
                ItemType::Asset => {
                    if let Some(asset) = self.release() {
                        dm.cancel_asset_download(asset);
                    }
                }
                ItemType::Docker => {
                    if let Some(v) = self.version() {
                        dm.cancel_docker_download(v);
                    }
                }
                ItemType::Epic => {
                    if let Some(v) = self.version() {
                        dm.cancel_epic_download(v);
                    }
                }
            }
        }
        self.remove_from_parent_with_timer(15);
    }

    fn pause(&self) {
        let self_ = self.imp();
        get_action!(self_.actions, @pause).set_enabled(false);
        glib::timeout_add_seconds_local(
            2,
            clone!(
                #[weak(rename_to=obj)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let self_ = obj.imp();
                    get_action!(self_.actions, @pause).set_enabled(true);
                    glib::ControlFlow::Break
                }
            ),
        );
        if let Some(dm) = self_.download_manager.get() {
            match self.item_type() {
                ItemType::Unknown => {}
                ItemType::Asset => {
                    if let Some(asset) = self.release() {
                        if self.paused() {
                            self_
                                .pause_button
                                .set_icon_name("media-playback-pause-symbolic");
                            dm.resume_asset_download(asset);
                        } else {
                            self_
                                .pause_button
                                .set_icon_name("media-playback-start-symbolic");
                            dm.pause_asset_download(asset);
                        }
                    }
                }
                ItemType::Docker => {
                    if let Some(v) = self.version() {
                        if self.paused() {
                            self_
                                .pause_button
                                .set_icon_name("media-playback-pause-symbolic");
                            dm.resume_docker_download(v);
                        } else {
                            self_
                                .pause_button
                                .set_icon_name("media-playback-start-symbolic");
                            dm.pause_docker_download(v);
                        }
                    }
                }
                ItemType::Epic => {
                    if let Some(v) = self.version() {
                        if self.paused() {
                            self_
                                .pause_button
                                .set_icon_name("media-playback-pause-symbolic");
                            dm.resume_epic_download(v);
                        } else {
                            self_
                                .pause_button
                                .set_icon_name("media-playback-start-symbolic");
                            dm.pause_epic_download(v);
                        }
                    }
                }
            }
        }
        self.set_property("paused", !self.paused());
    }

    pub fn setup_messaging(&self) {
        let _self_: &imp::EpicDownloadItem = self.imp();
    }

    pub fn set_total_size(&self, size: u128) {
        let self_ = self.imp();
        self_.total_size.replace(size);
    }

    pub fn total_size(&self) -> u128 {
        let self_ = self.imp();
        *self_.total_size.borrow()
    }

    pub fn downloaded_size(&self) -> u128 {
        let self_ = self.imp();
        *self_.downloaded_size.borrow()
    }

    pub fn path(&self) -> Option<String> {
        self.property("path")
    }

    pub fn target(&self) -> Option<String> {
        self.property("target")
    }

    pub fn asset(&self) -> Option<String> {
        self.property("asset")
    }

    pub fn version(&self) -> Option<String> {
        self.property("version")
    }

    pub fn release(&self) -> Option<String> {
        self.property("release")
    }

    pub fn paused(&self) -> bool {
        self.property("paused")
    }

    pub fn canceled(&self) -> bool {
        self.property("canceled")
    }

    pub fn item_type(&self) -> ItemType {
        self.property("item-type")
    }

    pub fn set_total_files(&self, count: u64) {
        let self_ = self.imp();
        self_.total_files.replace(count);
    }

    pub fn file_processed(&self) {
        let self_ = self.imp();
        if self.canceled() || self.paused() {
            return;
        }
        let new_count = *self_.extracted_files.borrow() + 1;
        let total = *self_.total_files.borrow();
        self_
            .extraction_progress
            .set_tooltip_text(Some(&format!("{new_count}/{total}")));
        self_
            .extraction_progress
            .set_fraction(new_count as f64 / total as f64);
        self_.extracted_files.replace(new_count);
        if new_count == total {
            {
                let queue = &mut *self_.speed_queue.borrow_mut();
                queue.clear();
            };
            self_.downloaded_size.replace(*self_.total_size.borrow());
            self_.download_progress.set_fraction(1.0);
            if let Some(w) = self_.window.get() {
                let w_ = w.imp();
                let l = w_.logged_in_stack.clone();
                let l_ = l.imp();
                if let Some(id) = self.asset() {
                    l_.library.refresh_asset(&id);
                }
            }
            self.set_property("status", "Finished".to_string());
            self.remove_from_parent_with_timer(5);
        };
    }

    fn remove_from_parent_with_timer(&self, timer: u32) {
        glib::timeout_add_seconds_local(
            timer,
            clone!(
                #[weak(rename_to=obj)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    obj.emit_by_name::<()>("finished", &[]);
                    glib::ControlFlow::Break
                }
            ),
        );
    }

    pub fn add_downloaded_size(&self, size: u128) {
        let self_ = self.imp();
        if self.canceled() || self.paused() {
            return;
        }
        get_action!(self_.actions, @cancel).set_enabled(true);
        match self.item_type() {
            ItemType::Unknown => {}
            ItemType::Asset | ItemType::Docker => {
                get_action!(self_.actions, @pause).set_enabled(true);
            }
            ItemType::Epic => {
                get_action!(self_.actions, @pause).set_enabled(false);
            }
        }
        // Download Speed
        {
            let queue = &mut *self_.speed_queue.borrow_mut();
            queue.push_back((chrono::Utc::now(), size));
        };

        let old_size = self.downloaded_size();
        let new_size = old_size + size;
        let total = self.total_size();
        self_
            .download_progress
            .set_fraction(new_size as f64 / total as f64);
        self_.downloaded_size.replace(new_size);
        if new_size == total {
            self_.download_progress.set_sensitive(false);
        }
    }

    pub fn add_extracted_size(&self, size: u128) {
        let self_ = self.imp();
        if self.canceled() || self.paused() {
            return;
        }
        // Extraction Speed
        {
            let queue = &mut *self_.speed_queue.borrow_mut();
            queue.push_back((chrono::Utc::now(), size));
        };
    }

    pub fn progress(&self) -> f32 {
        let self_ = self.imp();
        let new_size = *self_.downloaded_size.borrow();
        let total = *self_.total_size.borrow();
        let new_count = *self_.extracted_files.borrow();
        let total_count = *self_.total_files.borrow();
        ((if total == 0 {
            0.0
        } else if new_count == total_count {
            1.0
        } else {
            new_size as f32 / total as f32
        }) / 2.0)
            + ((if total_count == 0 {
                0.0
            } else {
                new_count as f32 / total_count as f32
            }) / 2.0)
    }

    pub fn add_actions(&self, act: &[super::PostDownloadAction]) {
        let self_ = self.imp();
        let mut current = self_.post_actions.borrow_mut();
        let mut result: Vec<PostDownloadAction> = Vec::new();
        for a in act {
            if current.contains(a) {
                continue;
            }
            result.push(a.clone());
        }
        current.append(&mut result);
    }

    pub fn actions(&self) -> Vec<PostDownloadAction> {
        let self_ = self.imp();
        self_.post_actions.borrow().clone()
    }

    pub fn open_path(&self) {
        let self_ = self.imp();
        if let Some(p) = self.path() {
            if let Some(w) = self_.window.get() {
                w.close_download_manager();
            }
            let ctx = glib::MainContext::default();
            ctx.spawn_local(async move {
                crate::tools::open_directory(&p).await;
            });
        };
    }
}
