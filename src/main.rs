extern crate glib;

use crate::glib::Cast;

extern crate webkit2gtk;

use crate::Msg::{
    LoadDownloadManifest, Login, LoginOk, ProcessAssetInfo, ProcessAssetList,
    ProcessDownloadManifest, ProcessImage,
};

use egs_api::api::UserData;
use egs_api::EpicGames;
use gio;
use gtk::Orientation::{Horizontal, Vertical};
use gtk::{
    prelude::BuilderExtManual, Align, Box, Builder, Button, ButtonExt, CellLayoutExt,
    CellRendererToggleExt, ComboBoxExt, ComboBoxText, ComboBoxTextExt, ContainerExt, EntryExt,
    FileChooserButton, FileChooserExt, FlowBox, FlowBoxChild, FlowBoxExt, GridBuilder, GridExt,
    GtkListStoreExt, IconSize, Image, ImageExt, Inhibit, Justification, Label, LabelExt,
    MenuButton, MenuButtonExt, Overlay, OverlayExt, PopoverMenu, ProgressBar, ProgressBarExt,
    Revealer, RevealerExt, SearchEntry, SearchEntryExt, Separator, Stack, StackExt, TreeModelExt,
    TreeViewColumnExt, TreeViewExt, WidgetExt, Window,
};
use relm::{connect, Channel, Relm, Update, Widget, WidgetTest};
use relm_derive::Msg;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{fmt, fs, str, thread};
use tokio::runtime::Runtime;
use webkit2gtk::{LoadEvent, WebResourceExt, WebView, WebViewExt};

use crate::configuration::Configuration;
use gdk_pixbuf::PixbufLoaderExt;
use threadpool::ThreadPool;

extern crate env_logger;

#[macro_use]
extern crate log;

mod configuration;
mod models;

mod api_data;
mod tools;

use tools::asset_info::Search;

use crate::api_data::ApiData;
use crate::tools::cache::Cache;

#[macro_use]
extern crate lazy_static;

use crate::models::row_data::RowData;
use crate::tools::image_stock::ImageExtCust;
use byte_unit::Byte;
use egs_api::api::types::asset_info::{AssetInfo, KeyImage, ReleaseInfo};
use egs_api::api::types::download_manifest::DownloadManifest;
use egs_api::api::types::epic_asset::EpicAsset;
use glib::{IsA, ToValue};
use gtk::prelude::{ComboBoxExtManual, GtkListStoreExtManual};
use std::iter::FromIterator;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

lazy_static! {
    static ref DATA: ApiData = ApiData::new();
    static ref MUTEX: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    static ref RUNNING: Arc<std::sync::RwLock<bool>> = Arc::new(std::sync::RwLock::new(true));
}

trait Or: Sized {
    fn or(self, other: Self) -> Self;
}

impl<'a> Or for &'a str {
    fn or(self, other: &'a str) -> &'a str {
        if self.is_empty() {
            other
        } else {
            self
        }
    }
}

impl<'a> Or for &'a String {
    fn or(self, other: &'a String) -> &'a String {
        if self.is_empty() {
            other
        } else {
            self
        }
    }
}

#[derive(Clone)]
struct Model {
    relm: Relm<Win>,
    epic_games: EpicGames,
    configuration: Configuration,
    asset_model: crate::models::asset_model::Model,
    selected_asset: Option<String>,
}

#[derive(Msg, Debug, Clone)]
enum Msg {
    Quit,
    WebViewLoadFinished(LoadEvent),
    Login(String),
    Relogin,
    LoginOk(UserData),
    ProcessAssetList(HashMap<String, Vec<String>>, HashMap<String, EpicAsset>),
    ProcessAssetInfo(AssetInfo),
    ProcessImage(Option<String>, Vec<u8>),
    DownloadImage(Option<String>, KeyImage),
    #[allow(dead_code)]
    LoadDownloadManifest(String, ReleaseInfo),
    ProcessDownloadManifest(String, DownloadManifest),
    ProcessAssetSelected,
    FilterNone,
    FilterSome(String),
    Search,
    ApplyFilter,
    BindAssetModel,
    PulseProgress,
    CloseDetails,
    NextImage,
    PrevImage,
    ShowSettings(bool),
    ShowAssetDownload(bool),
    DownloadVersionSelected,
    ToggleAssetDownloadDetails,
}

#[derive(Debug)]
#[repr(i32)]
enum AssetFilesColumns {
    Filename,
    Size,
    Download,
}

fn fixed_toggled<W: IsA<gtk::CellRendererToggle>>(
    model: &gtk::ListStore,
    _w: &W,
    path: gtk::TreePath,
) {
    let iter = model.get_iter(&path).unwrap();
    let mut fixed = model
        .get_value(&iter, AssetFilesColumns::Download as i32)
        .get_some::<bool>()
        .unwrap_or_else(|err| {
            panic!(
                "ListStore value for {:?} at path {}: {}",
                AssetFilesColumns::Download,
                path,
                err
            )
        });
    fixed = !fixed;
    model.set_value(&iter, AssetFilesColumns::Download as u32, &fixed.to_value());
}

impl fmt::Display for Msg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Msg::Quit => {
                write!(f, "Quit")
            }
            Msg::WebViewLoadFinished(_) => {
                write!(f, "WebViewLoadFinished")
            }
            Login(_) => {
                write!(f, "Login")
            }
            Msg::Relogin => {
                write!(f, "Relogin")
            }
            LoginOk(_) => {
                write!(f, "LoginOk")
            }
            ProcessAssetList(_, _) => {
                write!(f, "ProcessAssetList")
            }
            ProcessAssetInfo(_) => {
                write!(f, "ProcessAssetInfo")
            }
            ProcessImage(_, _) => {
                write!(f, "ProcessImage")
            }
            LoadDownloadManifest(_, _) => {
                write!(f, "LoadDownloadManifest")
            }
            ProcessDownloadManifest(_, _) => {
                write!(f, "ProcessDownloadManifest")
            }
            Msg::ProcessAssetSelected => {
                write!(f, "ProcessAssetSelected")
            }
            Msg::FilterNone => {
                write!(f, "FilterNone")
            }
            Msg::FilterSome(_) => {
                write!(f, "FilterSome")
            }
            Msg::Search => {
                write!(f, "Search")
            }
            Msg::ApplyFilter => {
                write!(f, "ApplyFilter")
            }
            Msg::BindAssetModel => {
                write!(f, "BindAssetModel")
            }
            Msg::PulseProgress => {
                write!(f, "PulseProgress")
            }
            Msg::CloseDetails => {
                write!(f, "CloseDetails")
            }
            Msg::DownloadImage(_, _) => {
                write!(f, "DownloadImage")
            }
            Msg::NextImage => {
                write!(f, "NextImage")
            }
            Msg::PrevImage => {
                write!(f, "PrevImage")
            }
            Msg::ShowSettings(_) => {
                write!(f, "ShowSettings")
            }
            Msg::ShowAssetDownload(_) => {
                write!(f, "ShowAssetDownload")
            }
            Msg::DownloadVersionSelected => {
                write!(f, "DownloadVersionSelected")
            }
            Msg::ToggleAssetDownloadDetails => {
                write!(f, "ToggleAssetDownloadDetails")
            }
        }
    }
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone)]
struct Widgets {
    window: Window,
    login_view: WebView,
    login_box: Box,
    main_stack: Stack,
    title_right_box: Box,
    progress_message: Label,
    asset_flow: FlowBox,
    search: SearchEntry,
    progress_revealer: Revealer,
    loading_progress: ProgressBar,
    details_revealer: Revealer,
    details_content: Box,
    close_details: Button,
    image_stack: Stack,
    logged_in_stack: Stack,
    settings_widgets: Settings,
    asset_download_widgets: AssetDownloadDetails,
    download_button: Button,
}

#[derive(Clone)]
struct Settings {
    directory_selectors: HashMap<String, FileChooserButton>,
}

#[derive(Clone)]
struct AssetDownloadDetails {
    asset_version_combo: ComboBoxText,
    asset_download_info_box: Box,
    asset_download_info_revealer_button: Button,
    asset_download_info_revealer: Revealer,
    asset_download_info_revealer_button_image: Image,
    download_asset_name: Label,
    asset_download_content: Box,
}

struct Win {
    model: Model,
    widgets: Widgets,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginResponse {
    #[serde(rename = "redirectUrl")]
    pub redirect_url: String,
    pub sid: String,
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(relm: &Relm<Self>, _: ()) -> Model {
        Model {
            relm: relm.clone(),
            epic_games: EpicGames::new(),
            configuration: Configuration::new(),
            asset_model: crate::models::asset_model::Model::new(),
            selected_asset: None,
        }
    }

    fn update(&mut self, event: Msg) {
        let start = std::time::Instant::now();
        match event.clone() {
            Msg::Quit => {
                if let Ok(mut w) = RUNNING.write() {
                    *w = false
                }
                gtk::main_quit()
            }
            Msg::WebViewLoadFinished(event) => match event {
                LoadEvent::Finished => {
                    let resource = match self.widgets.login_view.get_main_resource() {
                        None => {
                            return;
                        }
                        Some(r) => r,
                    };
                    if let Some(uri) = resource.get_uri() {
                        if uri.as_str() == "https://www.epicgames.com/id/api/redirect" {
                            let stream = self.model.relm.stream().clone();
                            let (_channel, sender) = Channel::new(move |s| {
                                if let Some(sid) = s {
                                    stream.emit(Login(sid));
                                }
                            });
                            resource.get_data(None::<&gio::Cancellable>, move |data| {
                                if let Ok(d) = data {
                                    if let Ok(sid_response) =
                                        serde_json::from_slice::<LoginResponse>(&d)
                                    {
                                        sender.send(Some(sid_response.sid.clone())).unwrap();
                                    }
                                }
                            });
                        } else {
                            &self.widgets.main_stack.set_visible_child_name("login_box");
                        }
                    }
                }
                _ => {}
            },
            Msg::Login(sid) => {
                self.widgets.progress_message.set_label("Login in progress");
                &self.widgets.main_stack.set_visible_child_name("progress");
                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |ud| {
                    if let Some(user_data) = ud {
                        stream.emit(LoginOk(user_data));
                    }
                });

                let s = sid.clone();
                let mut eg = self.model.epic_games.clone();
                thread::spawn(move || {
                    let start = std::time::Instant::now();
                    if let Some(exchange_token) =
                        Runtime::new().unwrap().block_on(eg.auth_sid(s.as_str()))
                    {
                        if Runtime::new()
                            .unwrap()
                            .block_on(eg.auth_code(exchange_token))
                        {
                            sender.send(Some(eg.user_details())).unwrap();
                        }
                    };
                    debug!(
                        "{:?} - Login requests took {:?}",
                        thread::current().id(),
                        start.elapsed()
                    );
                });
            }
            Msg::Relogin => {
                self.widgets.progress_message.set_label("Resuming session");
                self.model
                    .epic_games
                    .set_user_details(self.model.configuration.user_data.clone().unwrap());
                &self.widgets.main_stack.set_visible_child_name("progress");
                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |ud| {
                    if let Some(user_data) = ud {
                        stream.emit(LoginOk(user_data));
                    }
                });

                let mut eg = self.model.epic_games.clone();
                thread::spawn(move || {
                    let start = std::time::Instant::now();
                    if Runtime::new().unwrap().block_on(eg.login()) {
                        sender.send(Some(eg.user_details())).unwrap();
                    };
                    debug!(
                        "{:?} - Relogin requests took {:?}",
                        thread::current().id(),
                        start.elapsed()
                    );
                });
            }
            Msg::LoginOk(user_data) => {
                self.model.epic_games.set_user_details(user_data);
                self.model.configuration.user_data =
                    Some(self.model.epic_games.user_details().to_owned());
                self.model.configuration.save();
                self.widgets
                    .main_stack
                    .set_visible_child_name("logged_in_stack");

                let logout_button = Button::with_label("Logout");
                let logged_in_box = Box::new(Vertical, 0);
                logged_in_box.add(&logout_button);
                let login_name = MenuButton::new();
                let logout_menu = PopoverMenu::new();
                logout_menu.add(&logged_in_box);
                login_name.set_label(
                    &self
                        .model
                        .epic_games
                        .user_details()
                        .display_name
                        .unwrap()
                        .as_str(),
                );
                logged_in_box.show_all();
                login_name.set_popover(Some(&logout_menu));
                login_name.show_all();

                &self.widgets.title_right_box.add(&login_name);
                &self.widgets.title_right_box.show_all();

                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |(anm, am)| {
                    stream.emit(ProcessAssetList(anm, am));
                });

                let mut eg = self.model.epic_games.clone();
                thread::spawn(move || {
                    let start = std::time::Instant::now();
                    let assets = Runtime::new().unwrap().block_on(eg.list_assets());
                    let mut asset_namespace_map: HashMap<String, Vec<String>> = HashMap::new();
                    let mut asset_map: HashMap<String, EpicAsset> = HashMap::new();
                    for asset in assets {
                        asset.save(None, None);
                        match asset_namespace_map.get_mut(asset.namespace.as_str()) {
                            None => {
                                asset_namespace_map.insert(
                                    asset.namespace.clone(),
                                    vec![asset.catalog_item_id.clone()],
                                );
                            }
                            Some(existing) => {
                                existing.push(asset.catalog_item_id.clone());
                            }
                        };
                        asset_map.insert(asset.catalog_item_id.clone(), asset);
                    }
                    sender.send((asset_namespace_map, asset_map)).unwrap();
                    debug!(
                        "{:?} - Requesting EpicAssets took {:?}",
                        thread::current().id(),
                        start.elapsed()
                    );
                });
            }
            ProcessAssetList(anm, am) => {
                // TODO: Cache EpicAsset

                if let Ok(mut asset_namespace_map) = DATA.asset_namespace_map.write() {
                    asset_namespace_map.clear();
                    asset_namespace_map.extend(anm.clone())
                }
                if let Ok(mut asset_map) = DATA.asset_map.write() {
                    asset_map.clear();
                    asset_map.extend(am.clone())
                }
                self.widgets.loading_progress.set_fraction(0.0);
                self.widgets
                    .loading_progress
                    .set_pulse_step(1.0 / am.len() as f64);
                self.widgets.progress_revealer.set_reveal_child(true);
                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |ai| {
                    stream.emit(ProcessAssetInfo(ai));
                });

                let eg = self.model.epic_games.clone();
                let mut fa: Vec<EpicAsset> = Vec::from_iter(am.values().cloned());
                thread::spawn(move || {
                    let start = std::time::Instant::now();
                    let pool = ThreadPool::new(3);
                    fa.sort_by(|a, b| a.app_name.cmp(&b.app_name));
                    for ass in fa.clone() {
                        let mut e = eg.clone();
                        let s = sender.clone();
                        pool.execute(move || {
                            let start = std::time::Instant::now();
                            match AssetInfo::load_from_cache(ass.catalog_item_id.clone(), None) {
                                None => {
                                    if let Some(asset) =
                                        Runtime::new().unwrap().block_on(e.get_asset_info(ass))
                                    {
                                        asset.save(None, None);
                                        if let Ok(mut asset_info) = DATA.asset_info.write() {
                                            asset_info.insert(asset.id.clone(), asset.clone());
                                        }
                                        s.send(asset).unwrap();
                                    };
                                }
                                Some(asset) => {
                                    if let Ok(mut asset_info) = DATA.asset_info.write() {
                                        asset_info.insert(asset.id.clone(), asset.clone());
                                    }
                                    s.send(asset).unwrap();
                                }
                            };
                            debug!(
                                "{:?} - Asset Info loading took {:?}",
                                thread::current().id(),
                                start.elapsed()
                            );
                        });
                    }
                    debug!(
                        "{:?} - AssetInfo processing took {:?}",
                        thread::current().id(),
                        start.elapsed()
                    );
                });
                debug!("Login took {:?}", start.elapsed());
            }
            ProcessAssetInfo(asset) => {
                // TODO: Cache the asset info
                let mut found = false;
                if let Some(images) = asset.key_images.clone() {
                    for image in images {
                        if image.type_field.eq_ignore_ascii_case("Thumbnail")
                            || image.type_field.eq_ignore_ascii_case("DieselGameBox")
                        {
                            self.model
                                .relm
                                .stream()
                                .emit(Msg::DownloadImage(Some(asset.id.clone()), image.clone()));

                            found = true;
                            break;
                        }
                    }
                    if !found {
                        debug!("{:?}: {:#?}", asset.title, asset.key_images);
                        self.model.relm.stream().emit(Msg::PulseProgress);
                    }
                }
            }
            Msg::DownloadImage(id, image) => {
                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |(id, b)| {
                    stream.emit(ProcessImage(id, b));
                });

                thread::spawn(move || {
                    let start = std::time::Instant::now();
                    match image.load() {
                        None => {
                            if let Ok(response) = reqwest::blocking::get(image.url.clone()) {
                                if let Ok(b) = response.bytes() {
                                    image.save(Some(Vec::from(b.as_ref())), None);
                                    match id {
                                        None => {
                                            sender.send((None, Vec::from(b.as_ref()))).unwrap();
                                        }
                                        Some(i) => {
                                            let mut _data = MUTEX.lock().unwrap();
                                            sender.send((Some(i), Vec::from(b.as_ref()))).unwrap();
                                            thread::sleep(std::time::Duration::from_millis(100));
                                        }
                                    }
                                }
                            };
                        }
                        Some(b) => match id {
                            None => {
                                sender.send((None, b)).unwrap();
                            }
                            Some(i) => {
                                let mut _data = MUTEX.lock().unwrap();
                                sender.send((Some(i), b)).unwrap();
                                thread::sleep(std::time::Duration::from_millis(100));
                            }
                        },
                    }
                    debug!(
                        "{:?} - Image loading took {:?}",
                        thread::current().id(),
                        start.elapsed()
                    );
                });
            }
            ProcessImage(asset_id, image) => match asset_id {
                Some(id) => {
                    if let Ok(asset_info) = DATA.asset_info.read() {
                        if let Some(asset) = asset_info.get(&id) {
                            self.model.asset_model.append(&RowData::new(
                                asset.title.clone(),
                                asset.clone(),
                                image,
                            ));
                            self.model.relm.stream().emit(Msg::PulseProgress);
                        }
                    }
                }
                None => {
                    let gtkimage = Image::new();
                    let pixbuf_loader = gdk_pixbuf::PixbufLoader::new();

                    if image.len() > 0 {
                        pixbuf_loader.write(&image).unwrap();
                        pixbuf_loader.close().ok();
                        let pixbuf = pixbuf_loader.get_pixbuf().unwrap();
                        let width = pixbuf.get_width();
                        let height = pixbuf.get_height();
                        let width_percent = 300.0 / width as f64;
                        let height_percent = 300.0 / height as f64;
                        let percent = if height_percent < width_percent {
                            height_percent
                        } else {
                            width_percent
                        };
                        let desired = (width as f64 * percent, height as f64 * percent);
                        gtkimage.set_from_pixbuf(
                            pixbuf_loader
                                .get_pixbuf()
                                .unwrap()
                                .scale_simple(
                                    desired.0.round() as i32,
                                    desired.1.round() as i32,
                                    gdk_pixbuf::InterpType::Bilinear,
                                )
                                .as_ref(),
                        );
                        gtkimage.set_property_expand(true);
                        gtkimage.show_all();
                        self.widgets.image_stack.add(&gtkimage)
                    }
                }
            },
            LoadDownloadManifest(id, release_info) => {
                let asset = match DATA.asset_info.read() {
                    Ok(asset_map) => match asset_map.get(id.as_str()) {
                        None => {
                            return;
                        }
                        Some(a) => a.clone(),
                    },
                    Err(_) => {
                        return;
                    }
                };

                if let Ok(download_manifests) = DATA.download_manifests.read() {
                    if let Some(dm) = download_manifests
                        .get(release_info.id.clone().unwrap_or(id.clone()).as_str())
                    {
                        self.model
                            .relm
                            .stream()
                            .emit(ProcessDownloadManifest(id.clone(), dm.clone()));
                        return;
                    }
                };

                let stream = self.model.relm.stream().clone();
                let ri = release_info.clone();
                let (_channel, sender) = Channel::new(move |dm: DownloadManifest| {
                    if let Ok(mut download_manifests) = DATA.download_manifests.write() {
                        download_manifests.insert(ri.clone().id.unwrap_or(id.clone()), dm.clone());
                    }
                    stream.emit(ProcessDownloadManifest(id.clone(), dm));
                });

                let mut eg = self.model.epic_games.clone();
                thread::spawn(move || {
                    let start = std::time::Instant::now();
                    if let Some(manifest) = Runtime::new().unwrap().block_on(eg.get_asset_manifest(
                        None,
                        None,
                        Some(asset.namespace),
                        Some(asset.id),
                        Some(release_info.app_id.unwrap_or_default()),
                    )) {
                        for elem in manifest.elements {
                            for man in elem.manifests {
                                if let Ok(d) = Runtime::new()
                                    .unwrap()
                                    .block_on(eg.get_asset_download_manifest(man.clone()))
                                {
                                    sender.send(d).unwrap();
                                    break;
                                };
                            }
                        }
                    };
                    debug!(
                        "{:?} - Download Manifest requests took {:?}",
                        thread::current().id(),
                        start.elapsed()
                    );
                });
            }
            ProcessDownloadManifest(id, dm) => {
                if self.model.selected_asset == Some(id) {
                    let size_box = Box::new(Horizontal, 0);
                    let size = dm.get_total_size();
                    let size_label = Label::new(Some("Total Size: "));
                    size_box.add(&size_label);
                    size_label.set_halign(Align::Start);
                    let size_d = Label::new(Some(
                        &Byte::from_bytes(size)
                            .get_appropriate_unit(false)
                            .to_string(),
                    ));
                    size_d.set_halign(Align::Start);
                    size_box.add(&size_d);
                    self.widgets
                        .asset_download_widgets
                        .asset_download_info_box
                        .add(&size_box);
                    self.widgets
                        .asset_download_widgets
                        .asset_download_info_box
                        .show_all();
                    let files = dm.get_files();
                    let col_types: [glib::Type; 3] =
                        [glib::Type::STRING, glib::Type::STRING, glib::Type::BOOL];
                    let store = gtk::ListStore::new(&col_types);
                    for (filename, manifest) in files {
                        let values: [(u32, &dyn ToValue); 3] = [
                            (0, &filename),
                            (
                                1,
                                &Byte::from_bytes(
                                    manifest
                                        .file_chunk_parts
                                        .iter()
                                        .map(|chunk| chunk.size)
                                        .sum(),
                                )
                                .get_appropriate_unit(false)
                                .to_string(),
                            ),
                            (2, &false),
                        ];
                        store.set(&store.append(), &values);
                    }
                    let model = Rc::new(store);
                    let treeview = gtk::TreeView::with_model(&*model);
                    treeview.set_vexpand(true);
                    treeview.set_search_column(AssetFilesColumns::Filename as i32);

                    {
                        let renderer = gtk::CellRendererText::new();
                        let column = gtk::TreeViewColumn::new();
                        column.pack_start(&renderer, true);
                        column.set_title("Filename");
                        column.add_attribute(&renderer, "text", AssetFilesColumns::Filename as i32);
                        column.set_sort_column_id(AssetFilesColumns::Filename as i32);
                        treeview.append_column(&column);
                    }

                    {
                        let renderer = gtk::CellRendererText::new();
                        let column = gtk::TreeViewColumn::new();
                        column.pack_start(&renderer, true);
                        column.set_title("Size");
                        column.add_attribute(&renderer, "text", AssetFilesColumns::Size as i32);
                        column.set_sort_column_id(AssetFilesColumns::Size as i32);
                        treeview.append_column(&column);
                    }

                    {
                        let renderer = gtk::CellRendererToggle::new();
                        let model_clone = model.clone();
                        renderer
                            .connect_toggled(move |w, path| fixed_toggled(&model_clone, w, path));
                        let column = gtk::TreeViewColumn::new();
                        column.pack_start(&renderer, true);
                        column.set_title("Downloaded");
                        column.add_attribute(
                            &renderer,
                            "active",
                            AssetFilesColumns::Download as i32,
                        );
                        column.set_sizing(gtk::TreeViewColumnSizing::Fixed);
                        column.set_fixed_width(50);
                        treeview.append_column(&column);
                    }

                    self.widgets
                        .asset_download_widgets
                        .asset_download_content
                        .add(&treeview);
                    self.widgets
                        .asset_download_widgets
                        .asset_download_content
                        .show_all();
                }
            }
            Msg::ProcessAssetSelected => {
                for child in self.widgets.asset_flow.get_selected_children() {
                    if let Ok(ai) = DATA.asset_info.read() {
                        if let Some(asset_info) = ai.get(child.get_widget_name().as_str()) {
                            self.widgets
                                .details_content
                                .foreach(|el| self.widgets.details_content.remove(el));

                            info!("Showing details for {:?}", asset_info.title);
                            self.model.selected_asset = Some(asset_info.id.clone());

                            let vbox = Box::new(Vertical, 0);

                            if let Some(title) = &asset_info.title {
                                let name = Label::new(None);
                                name.set_markup(&format!("<b><u><big>{}</big></u></b>", title));
                                name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
                                name.set_line_wrap(true);
                                name.set_halign(Align::Start);
                                vbox.add(&name);
                            }

                            vbox.add(&Separator::new(Horizontal));
                            self.widgets
                                .image_stack
                                .foreach(|el| self.widgets.image_stack.remove(el));
                            let image_navigation = Overlay::new();
                            image_navigation.set_size_request(-1, 300);
                            let back = Button::with_label("<");
                            back.set_halign(Align::Start);
                            back.set_opacity(0.5);
                            connect!(self.model.relm, back, connect_clicked(_), Msg::PrevImage);
                            let forward = Button::with_label(">");
                            forward.set_opacity(0.5);
                            connect!(self.model.relm, forward, connect_clicked(_), Msg::NextImage);
                            forward.set_halign(Align::End);
                            image_navigation.add_overlay(&self.widgets.image_stack);
                            image_navigation.add_overlay(&back);
                            image_navigation.add_overlay(&forward);
                            vbox.add(&image_navigation);
                            if let Some(images) = &asset_info.key_images {
                                for image in images {
                                    if image.width < 300 || image.height < 300 {
                                        continue;
                                    }
                                    self.model
                                        .relm
                                        .stream()
                                        .emit(Msg::DownloadImage(None, image.clone()));
                                }
                            }
                            let table = GridBuilder::new()
                                .column_homogeneous(true)
                                .halign(Align::Start)
                                .valign(Align::Start)
                                .expand(false)
                                .build();
                            let developer_label = Label::new(Some("Developer:"));
                            developer_label.set_halign(Align::Start);
                            table.attach(&developer_label, 0, 0, 1, 1);
                            if let Some(dev_name) = &asset_info.developer {
                                let developer_name = Label::new(Some(dev_name));
                                developer_name.set_halign(Align::Start);
                                table.attach(&developer_name, 1, 0, 1, 1);
                            }
                            let platforms_label = Label::new(Some("Platforms:"));
                            platforms_label.set_halign(Align::Start);
                            table.attach(&platforms_label, 0, 1, 1, 1);
                            let platforms = Label::new(Some(
                                &asset_info.get_platforms().unwrap_or_default().join(", "),
                            ));
                            platforms.set_halign(Align::Start);
                            platforms.set_line_wrap(true);
                            table.attach(&platforms, 1, 1, 1, 1);
                            let comp_label = Label::new(Some("Compatible with:"));
                            comp_label.set_halign(Align::Start);

                            table.attach(&comp_label, 0, 2, 1, 1);
                            if let Some(comp) = &asset_info.get_compatible_apps() {
                                let compat = Label::new(Some(&comp.join(", ").replace("UE_", "")));
                                compat.set_halign(Align::Start);
                                compat.set_line_wrap(true);
                                table.attach(&compat, 1, 2, 1, 1);
                            }
                            vbox.add(&table);
                            vbox.add(&Separator::new(Horizontal));
                            if let Some(desc) = &asset_info.long_description {
                                let description = Label::new(None);
                                description.set_line_wrap(true);
                                let markup =
                                    html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n");
                                description.set_markup(&markup);
                                vbox.add(&description);
                            }
                            if let Some(desc) = &asset_info.technical_details {
                                let description = Label::new(None);
                                description.set_line_wrap(true);
                                let markup =
                                    html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n");
                                description.set_markup(&markup);
                                vbox.add(&description);
                            }
                            if asset_info.release_info.clone().unwrap().len() > 0 {
                                self.widgets.download_button.set_sensitive(true);
                            }

                            vbox.show_all();
                            self.widgets.details_content.add(&vbox);
                            self.widgets.details_revealer.set_reveal_child(true);
                        }
                    }
                }
            }
            Msg::FilterNone => {
                if let Ok(mut tag_filter) = DATA.tag_filter.write() {
                    *tag_filter = None;
                }
                self.model.relm.stream().emit(Msg::ApplyFilter);
            }
            Msg::FilterSome(filter) => {
                if let Ok(mut tag_filter) = DATA.tag_filter.write() {
                    tag_filter.replace(filter);
                }
                self.model.relm.stream().emit(Msg::ApplyFilter);
            }
            Msg::Search => {
                let search = self.widgets.search.get_text().to_string();
                if let Ok(mut search_filter) = DATA.search_filter.write() {
                    if search.is_empty() {
                        *search_filter = None;
                    } else {
                        search_filter.replace(search.clone());
                    }
                }
                self.model.relm.stream().emit(Msg::ApplyFilter);
            }
            Msg::ApplyFilter => {
                self.widgets
                    .asset_flow
                    .set_filter_func(Some(std::boxed::Box::new(|child| -> bool {
                        let id = child.get_widget_name().to_string();
                        match DATA.asset_info.read() {
                            Ok(asset_info) => match asset_info.get(&id) {
                                Some(ai) => {
                                    let tag = match DATA.tag_filter.read() {
                                        Ok(tag_filter) => tag_filter.clone(),
                                        Err(_) => None,
                                    };
                                    let search = match DATA.search_filter.read() {
                                        Ok(search_filter) => search_filter.clone(),
                                        Err(_) => None,
                                    };
                                    ai.matches_filter(tag, search)
                                }
                                None => false,
                            },
                            Err(_) => false,
                        }
                    })));
            }
            Msg::BindAssetModel => {
                self.widgets
                    .asset_flow
                    .bind_model(Some(&self.model.asset_model), |asset| {
                        let start = std::time::Instant::now();
                        let child = FlowBoxChild::new();
                        let object = asset
                            .downcast_ref::<crate::models::row_data::RowData>()
                            .unwrap();
                        let data: AssetInfo = object.deserialize();
                        child.set_widget_name(&data.id);
                        let image = object.image();
                        let vbox = Box::new(Vertical, 0);
                        let gtkimage = Image::new();
                        if let Some(title) = &data.title {
                            gtkimage.set_tooltip_text(Some(title));
                        }

                        let pixbuf_loader = gdk_pixbuf::PixbufLoader::new();

                        if image.len() > 0 {
                            pixbuf_loader.write(&image).unwrap();
                            pixbuf_loader.close().ok();
                            gtkimage.set_from_pixbuf(
                                pixbuf_loader
                                    .get_pixbuf()
                                    .unwrap()
                                    .scale_simple(128, 128, gdk_pixbuf::InterpType::Bilinear)
                                    .as_ref(),
                            );
                        }
                        vbox.set_size_request(130, 150);
                        vbox.add(&gtkimage);
                        if let Some(title) = &data.title {
                            let label = Label::new(Some(title));
                            label.set_property_wrap(true);
                            label.set_property_expand(false);
                            label.set_max_width_chars(15);
                            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                            label.set_tooltip_text(Some(title));
                            label.set_justify(Justification::Center);
                            vbox.add(&label);
                        }

                        vbox.set_property_margin(10);
                        child.add(&vbox);
                        vbox.show_all();
                        debug!(
                            "{:?} - building a model widget took {:?}",
                            thread::current().id(),
                            start.elapsed()
                        );
                        child.upcast::<gtk::Widget>()
                    });
            }
            Msg::PulseProgress => {
                self.widgets.loading_progress.set_fraction(
                    self.widgets.loading_progress.get_fraction()
                        + self.widgets.loading_progress.get_pulse_step(),
                );
                if (self.widgets.loading_progress.get_fraction() * 10000.0).round() / 10000.0 == 1.0
                {
                    debug!("Hiding progress");
                    self.widgets.progress_revealer.set_reveal_child(false);
                }
            }
            Msg::CloseDetails => {
                self.widgets.download_button.set_sensitive(false);
                self.widgets.details_revealer.set_reveal_child(false);
                self.widgets.asset_flow.unselect_all();
            }
            Msg::NextImage => {
                let total = self.widgets.image_stack.get_children().len() as i32;
                if total > 0 {
                    let current = self.widgets.image_stack.get_visible_child().unwrap();
                    let pos = self.widgets.image_stack.get_child_position(&current);

                    if pos + 1 >= total {
                        if let Some(new) = self.widgets.image_stack.get_children().get(0) {
                            self.widgets.image_stack.set_visible_child(new);
                        }
                    } else {
                        if let Some(new) = self
                            .widgets
                            .image_stack
                            .get_children()
                            .get((pos + 1) as usize)
                        {
                            self.widgets.image_stack.set_visible_child(new);
                        }
                    };
                }
            }
            Msg::PrevImage => {
                let total = self.widgets.image_stack.get_children().len() as i32;
                if total > 0 {
                    let current = self.widgets.image_stack.get_visible_child().unwrap();
                    let pos = self.widgets.image_stack.get_child_position(&current);
                    if pos - 1 < 0 {
                        if let Some(new) = self.widgets.image_stack.get_children().last() {
                            self.widgets.image_stack.set_visible_child(new);
                        }
                    } else {
                        if let Some(new) = self
                            .widgets
                            .image_stack
                            .get_children()
                            .get((pos - 1) as usize)
                        {
                            self.widgets.image_stack.set_visible_child(new);
                        }
                    };
                }
            }
            Msg::ShowSettings(enabled) => {
                self.widgets
                    .logged_in_stack
                    .set_visible_child_name(if enabled { "settings" } else { "main" });
            }
            Msg::ShowAssetDownload(enabled) => {
                // Cleanup
                self.widgets
                    .asset_download_widgets
                    .asset_version_combo
                    .remove_all();

                if enabled {
                    if let Some(asset_id) = &self.model.selected_asset {
                        if let Ok(ai) = DATA.asset_info.read() {
                            if let Some(asset_info) = ai.get(asset_id) {
                                self.widgets
                                    .asset_download_widgets
                                    .download_asset_name
                                    .set_markup(&format!(
                                        "<b><u><big>{}</big></u></b>",
                                        asset_info.title.clone().unwrap_or("Nothing".to_string())
                                    ));
                                if let Some(releases) = asset_info.get_sorted_releases() {
                                    for (id, release) in releases.iter().enumerate() {
                                        self.widgets
                                            .asset_download_widgets
                                            .asset_version_combo
                                            .append(
                                                Some(
                                                    release.id.as_ref().unwrap_or(&"".to_string()),
                                                ),
                                                &format!(
                                                    "{}{}",
                                                    release
                                                        .version_title
                                                        .as_ref()
                                                        .unwrap_or(&"".to_string())
                                                        .or(release
                                                            .app_id
                                                            .as_ref()
                                                            .unwrap_or(&"".to_string())),
                                                    if id == 0 { " (latest)" } else { "" }
                                                ),
                                            )
                                    }
                                    self.widgets
                                        .asset_download_widgets
                                        .asset_version_combo
                                        .set_active(Some(0));
                                }
                            };
                        };
                    };
                };

                self.widgets
                    .logged_in_stack
                    .set_visible_child_name(if enabled {
                        "asset_download_details"
                    } else {
                        "main"
                    });
            }
            Msg::DownloadVersionSelected => {
                if let Some(id) = self
                    .widgets
                    .asset_download_widgets
                    .asset_version_combo
                    .get_active_id()
                {
                    if let Some(asset_id) = &self.model.selected_asset {
                        if let Ok(ai) = DATA.asset_info.read() {
                            if let Some(asset_info) = ai.get(asset_id) {
                                if !self
                                    .widgets
                                    .asset_download_widgets
                                    .asset_download_info_revealer
                                    .get_reveal_child()
                                {
                                    self.model
                                        .relm
                                        .stream()
                                        .emit(Msg::ToggleAssetDownloadDetails)
                                }
                                self.widgets
                                    .asset_download_widgets
                                    .asset_download_info_box
                                    .foreach(|el| {
                                        self.widgets
                                            .asset_download_widgets
                                            .asset_download_info_box
                                            .remove(el)
                                    });
                                self.widgets
                                    .asset_download_widgets
                                    .asset_download_content
                                    .foreach(|el| {
                                        self.widgets
                                            .asset_download_widgets
                                            .asset_download_content
                                            .remove(el)
                                    });
                                let grid = GridBuilder::new()
                                    .column_homogeneous(true)
                                    .halign(Align::Start)
                                    .valign(Align::Start)
                                    .expand(false)
                                    .build();
                                if let Some(release) = asset_info.get_release_id(id.to_string()) {
                                    let mut line = 0;
                                    if let Some(ref compatible) = release.compatible_apps {
                                        let versions_label =
                                            Label::new(Some("Supported versions:"));
                                        versions_label.set_halign(Align::Start);
                                        grid.attach(&versions_label, 0, line, 1, 1);
                                        let compat = Label::new(Some(
                                            &compatible.join(", ").replace("UE_", ""),
                                        ));
                                        compat.set_halign(Align::Start);
                                        compat.set_line_wrap(true);
                                        grid.attach(&compat, 1, line, 1, 1);
                                        line += 1;
                                    }
                                    if let Some(ref platforms) = release.platform {
                                        let platforms_label = Label::new(Some("Platforms:"));
                                        platforms_label.set_halign(Align::Start);
                                        grid.attach(&platforms_label, 0, line, 1, 1);
                                        let platforms = Label::new(Some(&platforms.join(", ")));
                                        platforms.set_halign(Align::Start);
                                        grid.attach(&platforms, 1, line, 1, 1);
                                        line += 1;
                                    }
                                    if let Some(ref date) = release.date_added {
                                        let release_date_label = Label::new(Some("Release date:"));
                                        release_date_label.set_halign(Align::Start);
                                        grid.attach(&release_date_label, 0, line, 1, 1);
                                        let release_date = Label::new(Some(
                                            &date.naive_local().format("%F").to_string(),
                                        ));
                                        release_date.set_halign(Align::Start);
                                        grid.attach(&release_date, 1, line, 1, 1);
                                        line += 1;
                                    }
                                    if let Some(ref note) = release.release_note {
                                        if !note.is_empty() {
                                            let release_note_label =
                                                Label::new(Some("Release note:"));
                                            release_note_label.set_halign(Align::Start);
                                            grid.attach(&release_note_label, 0, line, 1, 1);
                                            let release_note = Label::new(Some(&note));
                                            release_note.set_halign(Align::Start);
                                            grid.attach(&release_note, 1, line, 1, 1);
                                        };
                                    }

                                    grid.show_all();
                                    self.widgets
                                        .asset_download_widgets
                                        .asset_download_info_box
                                        .add(&grid);
                                    self.model.relm.stream().emit(Msg::LoadDownloadManifest(
                                        asset_info.id.clone(),
                                        release,
                                    ));
                                };
                            }
                        }
                    }
                }
            }
            Msg::ToggleAssetDownloadDetails => {
                self.widgets
                    .asset_download_widgets
                    .asset_download_info_revealer_button_image
                    .set_from_stock(
                        if self
                            .widgets
                            .asset_download_widgets
                            .asset_download_info_revealer
                            .get_reveal_child()
                        {
                            Some("gtk-go-down")
                        } else {
                            Some("gtk-go-up")
                        },
                        IconSize::Button,
                    );

                self.widgets
                    .asset_download_widgets
                    .asset_download_info_revealer
                    .set_reveal_child(
                        !self
                            .widgets
                            .asset_download_widgets
                            .asset_download_info_revealer
                            .get_reveal_child(),
                    )
            }
        }
        debug!(
            "{:?} - {} took {:?}",
            thread::current().id(),
            event,
            start.elapsed()
        );
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = Window;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        debug!("Main thread id: {:?}", thread::current().id());

        info!("Starging GTK Window");
        let glade_src = include_str!("gui.glade");
        let builder = Builder::from_string(glade_src);

        let window: Window = builder.get_object("window").unwrap();
        let main_stack: Stack = builder.get_object("main_stack").unwrap();
        let logged_in_stack: Stack = builder.get_object("logged_in_stack").unwrap();
        let login_box: Box = builder.get_object("login_box").unwrap();
        let title_right_box: Box = builder.get_object("title_right_box").unwrap();
        let progress_message = builder.get_object("progress_message").unwrap();
        let asset_flow: FlowBox = builder.get_object("asset_flow").unwrap();
        let all_button: Button = builder.get_object("all_button").unwrap();
        let assets_button: Button = builder.get_object("assets_button").unwrap();
        let plugins_button: Button = builder.get_object("plugins_button").unwrap();
        let games_button: Button = builder.get_object("games_button").unwrap();
        let settings_button: Button = builder.get_object("settings_button").unwrap();
        let search: SearchEntry = builder.get_object("search").unwrap();
        let progress_revealer: Revealer = builder.get_object("progress_revealer").unwrap();
        let loading_progress: ProgressBar = builder.get_object("loading_progress").unwrap();
        let details_revealer: Revealer = builder.get_object("details_revealer").unwrap();
        let details_content: Box = builder.get_object("details_content").unwrap();
        let close_details: Button = builder.get_object("close_details").unwrap();
        let settings_close: Button = builder.get_object("settings_close").unwrap();
        let download_button: Button = builder.get_object("download_button").unwrap();
        let asset_download_details_close: Button =
            builder.get_object("asset_download_details_close").unwrap();

        let image_stack = Stack::new();

        relm.stream().emit(Msg::BindAssetModel);

        connect!(relm, search, connect_search_changed(_), Msg::Search);

        connect!(relm, all_button, connect_clicked(_), Msg::FilterNone);
        connect!(
            relm,
            settings_button,
            connect_clicked(_),
            Msg::ShowSettings(true)
        );
        connect!(
            relm,
            settings_close,
            connect_clicked(_),
            Msg::ShowSettings(false)
        );
        connect!(
            relm,
            download_button,
            connect_clicked(_),
            Msg::ShowAssetDownload(true)
        );
        connect!(
            relm,
            asset_download_details_close,
            connect_clicked(_),
            Msg::ShowAssetDownload(false)
        );
        connect!(
            relm,
            assets_button,
            connect_clicked(_),
            Msg::FilterSome("assets".to_string())
        );
        connect!(
            relm,
            plugins_button,
            connect_clicked(_),
            Msg::FilterSome("plugins".to_string())
        );
        connect!(
            relm,
            games_button,
            connect_clicked(_),
            Msg::FilterSome("games".to_string())
        );
        connect!(relm, close_details, connect_clicked(_), Msg::CloseDetails);

        let webview = WebView::new();
        webview.set_property_expand(true);
        connect!(
            relm,
            webview,
            connect_load_changed(_, a),
            Msg::WebViewLoadFinished(a)
        );
        login_box.add(&webview);

        match model.configuration.user_data {
            None => {
                webview.load_uri("https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect");
            }
            Some(_) => {
                relm.stream().emit(Msg::Relogin);
            }
        }
        connect!(
            relm,
            window,
            connect_delete_event(_, _),
            return (Some(Msg::Quit), Inhibit(false))
        );
        connect!(
            relm,
            asset_flow,
            connect_selected_children_changed(_),
            Msg::ProcessAssetSelected
        );

        // Settings
        let mut directory_selectors: HashMap<String, FileChooserButton> = HashMap::new();

        directory_selectors.insert(
            "cache_directory_selector".into(),
            builder.get_object("cache_directory_selector").unwrap(),
        );
        directory_selectors.insert(
            "temp_download_directory_selector".into(),
            builder
                .get_object("temp_download_directory_selector")
                .unwrap(),
        );
        directory_selectors.insert(
            "ue_asset_vault_directory_selector".into(),
            builder
                .get_object("ue_asset_vault_directory_selector")
                .unwrap(),
        );

        directory_selectors.insert(
            "ue_directory_selector".into(),
            builder.get_object("ue_directory_selector").unwrap(),
        );

        fs::create_dir_all(&model.configuration.directories.cache_directory).unwrap();
        directory_selectors
            .get("cache_directory_selector")
            .unwrap()
            .set_filename(&model.configuration.directories.cache_directory);

        fs::create_dir_all(&model.configuration.directories.temporary_download_directory).unwrap();
        directory_selectors
            .get("temp_download_directory_selector")
            .unwrap()
            .set_filename(&model.configuration.directories.temporary_download_directory);

        fs::create_dir_all(&model.configuration.directories.unreal_vault_directory).unwrap();
        directory_selectors
            .get("ue_asset_vault_directory_selector")
            .unwrap()
            .set_filename(&model.configuration.directories.unreal_vault_directory);

        let settings_widgets = Settings {
            directory_selectors,
        };

        let asset_version_combo: ComboBoxText = builder.get_object("asset_version_combo").unwrap();

        let asset_download_info_box: Box = builder.get_object("asset_download_info_box").unwrap();
        let asset_download_content: Box = builder.get_object("asset_download_content").unwrap();
        let download_asset_name: Label = builder.get_object("download_asset_name").unwrap();
        let asset_download_info_revealer_button: Button = builder
            .get_object("asset_download_info_revealer_button")
            .unwrap();
        let asset_download_info_revealer: Revealer =
            builder.get_object("asset_download_info_revealer").unwrap();
        let asset_download_info_revealer_button_image: Image = builder
            .get_object("asset_download_info_revealer_button_image")
            .unwrap();
        connect!(
            relm,
            asset_version_combo,
            connect_changed(_),
            Msg::DownloadVersionSelected
        );
        connect!(
            relm,
            asset_download_info_revealer_button,
            connect_clicked(_),
            Msg::ToggleAssetDownloadDetails
        );
        let asset_download_widgets = AssetDownloadDetails {
            asset_download_content,
            download_asset_name,
            asset_version_combo,
            asset_download_info_box,
            asset_download_info_revealer_button,
            asset_download_info_revealer_button_image,
            asset_download_info_revealer,
        };

        window.show_all();

        Win {
            model,
            widgets: Widgets {
                logged_in_stack,
                main_stack,
                title_right_box,
                login_box,
                window,
                login_view: webview,
                progress_message,
                asset_flow,
                search,
                progress_revealer,
                loading_progress,
                details_revealer,
                details_content,
                close_details,
                image_stack,
                download_button,
                settings_widgets,
                asset_download_widgets,
            },
        }
    }
}

impl WidgetTest for Win {
    type Streams = ();

    fn get_streams(&self) -> Self::Streams {}

    type Widgets = Widgets;

    fn get_widgets(&self) -> Self::Widgets {
        self.widgets.clone()
    }
}

fn main() {
    Win::run(()).expect("Win::run failed");
}
