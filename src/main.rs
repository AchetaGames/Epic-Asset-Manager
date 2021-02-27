#[macro_use]
extern crate glib;

use crate::glib::Cast;

extern crate webkit2gtk;

use gtk::{prelude::BuilderExtManual, Builder, Button, ButtonExt, Inhibit, WidgetExt, Window, Box, ContainerExt, Stack, StackExt, MenuButton, MenuButtonExt, PopoverMenu, Label, LabelExt, ModelButton, FlowBox, Image, ImageExt, Align, FlowBoxChild, FlowBoxChildExt, BuildableExt, FlowBoxExt};
use relm_derive::Msg;
use relm::{connect, Relm, Update, Widget, WidgetTest, Channel};
use webkit2gtk::{WebView, WebViewExt, WebResourceExt, WebResource, LoadEvent};
use gio;
use std::{str, thread};
use serde::{Deserialize, Serialize};
use egs_api::EpicGames;
use tokio::runtime::Runtime;
use egs_api::api::UserData;
use std::collections::HashMap;
use egs_api::api::types::{EpicAsset, AssetInfo, DownloadManifest};
use crate::Msg::{LoginOk, Login, ProcessAssetList, ProcessAssetInfo, ProcessImage, LoadDownloadManifest, ProcessDownloadManifest};
use gtk::Orientation::{Vertical, Horizontal};

use std::io::Write;
use crate::configuration::Configuration;
use threadpool::ThreadPool;
use gdk_pixbuf::PixbufLoaderExt;
use std::cmp::Ordering;
use std::sync::{Arc, RwLock, PoisonError, RwLockWriteGuard};
use std::collections::hash_map::RandomState;
use std::rc::Rc;
use std::cell::RefCell;
use crate::models::asset_model::AssetModel;
use crate::models::ObjectWrapper;

extern crate env_logger;
#[macro_use]
extern crate log;

mod configuration;
mod models;

#[derive(Clone)]
struct Model {
    relm: Relm<Win>,
    epic_games: EpicGames,
    configuration: Configuration,
    asset_map: HashMap<String, EpicAsset>,
    asset_info: HashMap<String, AssetInfo>,
    asset_namespace_map: HashMap<String, Vec<String>>,
    download_manifests: HashMap<String, DownloadManifest>,
    asset_model: Rc<RefCell<AssetModel>>,
    asset_thumbnails: HashMap<String, Image>,
}

#[derive(Msg)]
enum Msg {
    Quit,
    WebViewLoadFinished(LoadEvent),
    Login(String),
    Relogin,
    LoginOk(UserData),
    ProcessAssetList(HashMap<String, Vec<String>>, HashMap<String, EpicAsset>),
    ProcessAssetInfo(AssetInfo),
    ProcessImage((String, Vec<u8>)),
    LoadDownloadManifest(String),
    ProcessDownloadManifest(String, DownloadManifest),
    ProcessAssetSelected,
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone)]
struct Widgets {
    window: Window,
    login_view: WebView,
    login_box: Box,
    main_stack: Stack,
    logged_in_box: Box,
    progress_message: Label,
    asset_flow: FlowBox,

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
            asset_map: Default::default(),
            asset_info: HashMap::new(),
            asset_namespace_map: Default::default(),
            download_manifests: Default::default(),
            asset_model: Rc::new(RefCell::new(AssetModel::new())),
            asset_thumbnails: Default::default(),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Quit => {
                gtk::main_quit()
            }
            Msg::WebViewLoadFinished(event) => {
                match event {
                    LoadEvent::Finished => {
                        let resource = match self.widgets.login_view.get_main_resource() {
                            None => { return; }
                            Some(r) => { r }
                        };
                        match resource.get_uri() {
                            None => {}
                            Some(uri) => {
                                if uri.as_str() == "https://www.epicgames.com/id/api/redirect" {
                                    let stream = self.model.relm.stream().clone();
                                    let (_channel, sender) = Channel::new(move |s| {
                                        match s {
                                            None => {}
                                            Some(sid) => { stream.emit(Login(sid)); }
                                        }
                                    });
                                    resource.get_data(None::<&gio::Cancellable>, move |data| {
                                        match data {
                                            Ok(d) => {
                                                match serde_json::from_slice::<LoginResponse>(&d) {
                                                    Ok(sid_response) => {
                                                        sender.send(Some(sid_response.sid.clone())).unwrap();
                                                    }
                                                    Err(_) => {}
                                                }
                                            }
                                            Err(_) => {}
                                        }
                                    });
                                } else {
                                    &self.widgets.main_stack.set_visible_child_name("login_box");
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Msg::Login(sid) => {
                self.widgets.progress_message.set_label("Login in progress");
                &self.widgets.main_stack.set_visible_child_name("progress");
                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |ud| {
                    match ud {
                        None => {}
                        Some(user_data) => { stream.emit(LoginOk(user_data)); }
                    }
                });

                let s = sid.clone();
                let mut eg = self.model.epic_games.clone();
                thread::spawn(move || {
                    match Runtime::new().unwrap().block_on(eg.auth_sid(s.as_str()))
                    {
                        None => {}
                        Some(exchange_token) => {
                            if Runtime::new().unwrap().block_on(eg.auth_code(exchange_token))
                            {
                                sender.send(Some(eg.user_details())).unwrap();
                            }
                        }
                    };
                });
            }
            Msg::Relogin => {
                self.widgets.progress_message.set_label("Resuming session");
                self.model.epic_games.set_user_details(self.model.configuration.user_data.clone().unwrap());
                &self.widgets.main_stack.set_visible_child_name("progress");
                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |ud| {
                    match ud {
                        None => {}
                        Some(user_data) => { stream.emit(LoginOk(user_data)); }
                    }
                });

                let mut eg = self.model.epic_games.clone();
                thread::spawn(move || {
                    if Runtime::new().unwrap().block_on(eg.login())
                    {
                        sender.send(Some(eg.user_details())).unwrap();
                    };
                });
            }
            Msg::LoginOk(user_data) => {
                self.model.epic_games.set_user_details(user_data);
                self.model.configuration.user_data = Some(self.model.epic_games.user_details().to_owned());
                self.model.configuration.save();
                self.widgets.main_stack.set_visible_child_name("logged_in_box");


                let logout_button = Button::with_label("Logout");
                let logged_in_box = Box::new(Vertical, 0);
                logged_in_box.add(&logout_button);
                let login_name = MenuButton::new();
                let logout_menu = PopoverMenu::new();
                logout_menu.add(&logged_in_box);
                login_name.set_label(&self.model.epic_games.user_details().display_name.unwrap().as_str());
                logged_in_box.show_all();
                // TODO: Does not show the contents of the popover
                login_name.set_popover(Some(&logout_menu));
                login_name.show_all();

                &self.widgets.logged_in_box.add(&login_name);
                &self.widgets.logged_in_box.show_all();

                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |(anm, am)| {
                    stream.emit(ProcessAssetList(anm, am));
                });

                let mut eg = self.model.epic_games.clone();
                thread::spawn(move || {
                    let assets = Runtime::new().unwrap().block_on(eg.list_assets());
                    let mut asset_namespace_map: HashMap<String, Vec<String>> = HashMap::new();
                    let mut asset_map: HashMap<String, EpicAsset> = HashMap::new();
                    for asset in assets {
                        match asset_namespace_map.get_mut(asset.namespace.as_str()) {
                            None => { asset_namespace_map.insert(asset.namespace.clone(), vec!(asset.catalog_item_id.clone())); }
                            Some(existing) => { existing.push(asset.catalog_item_id.clone()); }
                        };
                        asset_map.insert(asset.catalog_item_id.clone(), asset);
                    };
                    sender.send((asset_namespace_map, asset_map)).unwrap();
                });
            }
            ProcessAssetList(anm, asset_map) => {
                // TODO: Cache EpicAsset
                self.model.asset_namespace_map = anm.clone();
                self.model.asset_map = asset_map.clone();
                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |ai| {
                    stream.emit(ProcessAssetInfo(ai));
                });

                let eg = self.model.epic_games.clone();
                let fa = asset_map.clone();
                thread::spawn(move || {
                    let pool = ThreadPool::new(3);
                    for (_, ass) in fa.clone() {
                        let mut e = eg.clone();
                        let s = sender.clone();
                        pool.execute(move || {
                            if let Some(asset) = Runtime::new().unwrap().block_on(e.get_asset_info(ass)) {
                                s.send(asset).unwrap();
                            };
                        });
                    }
                });
            }
            ProcessAssetInfo(asset) => {
                // TODO: Cache the asset info
                self.model.asset_info.insert(asset.id.clone(), asset.clone());

                let mut found = false;
                for image in asset.key_images.clone() {
                    if image.type_field.eq_ignore_ascii_case("Thumbnail") || image.type_field.eq_ignore_ascii_case("DieselGameBox") {
                        let stream = self.model.relm.stream().clone();
                        let (_channel, sender) = Channel::new(move |(id, b)| {
                            stream.emit(ProcessImage((id, b)));
                        });

                        let id = asset.id.clone();
                        thread::spawn(move || {
                            match reqwest::blocking::get(&image.url) {
                                Ok(response) => {
                                    match response.bytes() {
                                        Ok(b) => {
                                            sender.send((id, Vec::from(b.as_ref()))).unwrap();
                                        }
                                        Err(_) => {}
                                    }
                                }
                                Err(_) => {}
                            };
                        });
                        found = true;
                        break;
                    }
                }
                if !found {
                    println!("{}: {:#?}", asset.title, asset.key_images)
                }
            }
            ProcessImage((id, image)) => {
                match self.model.asset_info.get(&id) {
                    None => {}
                    Some(asset) => {
                        self.model.asset_model.borrow_mut().add_asset(asset.clone(), image);
                    }
                }
            }
            LoadDownloadManifest(id) => {
                let asset = match self.model.asset_map.get(id.as_str())
                {
                    None => { return; }
                    Some(a) => { a.clone() }
                };

                if let Some(_) = self.model.download_manifests.get(id.as_str()) { return; }
                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |dm| {
                    stream.emit(ProcessDownloadManifest(id.clone(), dm));
                });

                let mut eg = self.model.epic_games.clone();
                thread::spawn(move || {
                    match Runtime::new().unwrap().block_on(eg.get_asset_manifest(asset)) {
                        None => {}
                        Some(manifest) => {
                            for elem in manifest.elements {
                                for man in elem.manifests {
                                    match Runtime::new().unwrap().block_on(eg.get_asset_download_manifest(man.clone())) {
                                        Ok(d) => {
                                            sender.send(d).unwrap();
                                            break;
                                        }
                                        Err(_) => {}
                                    };
                                }
                            }
                        }
                    };
                });
            }
            ProcessDownloadManifest(id, dm) => {
                self.model.download_manifests.insert(id.clone(), dm.clone());
            }
            Msg::ProcessAssetSelected => {
                self.widgets.asset_flow.selected_foreach(
                    |_fbox, child| {
                        println!("Selected {}", child.get_widget_name().to_string());
                    }
                );
            }
        }
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
        let glade_src = include_str!("gui.glade");
        let builder = Builder::from_string(glade_src);

        let window: Window = builder.get_object("window").unwrap();
        let main_stack: Stack = builder.get_object("main_stack").unwrap();
        let login_box: Box = builder.get_object("login_box").unwrap();
        let logged_in_box: Box = builder.get_object("title_right_box").unwrap();
        let progress_message = builder.get_object("progress_message").unwrap();
        let asset_flow: FlowBox = builder.get_object("asset_flow").unwrap();

        asset_flow.bind_model(Some(&model.asset_model.borrow().model),
                              move |asset| {
                                  let child = FlowBoxChild::new();
                                  let object = asset.downcast_ref::<ObjectWrapper>().unwrap();
                                  let data: AssetInfo = object.deserialize();
                                  let image = object.image();
                                  let vbox = Box::new(Vertical, 0);
                                  let gtkimage = Image::new();
                                  let pixbuf_loader = gdk_pixbuf::PixbufLoader::new();
                                  child.set_widget_name(&data.id);
                                  if image.len() > 0 {
                                      pixbuf_loader.write(&image).unwrap();
                                      pixbuf_loader.close().ok();
                                      gtkimage.set_from_pixbuf(pixbuf_loader.get_pixbuf().unwrap().scale_simple(128, 128, gdk_pixbuf::InterpType::Bilinear).as_ref());
                                  }
                                  vbox.set_size_request(130, -1);
                                  vbox.add(&gtkimage);
                                  let label = Label::new(Some(&data.title));
                                  label.set_property_wrap(true);
                                  label.set_property_expand(false);
                                  label.set_max_width_chars(15);
                                  label.set_halign(Align::Center);
                                  vbox.add(&label);
                                  child.add(&vbox);
                                  vbox.show_all();
                                  child.upcast::<gtk::Widget>()
                              },
        );

        let webview = WebView::new();
        webview.set_property_expand(true);
        connect!(relm, webview, connect_load_changed(_,a), Msg::WebViewLoadFinished(a) );
        login_box.add(&webview);

        match model.configuration.user_data {
            None => { webview.load_uri("https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect"); }
            Some(_) => {
                relm.stream().emit(Msg::Relogin);
            }
        }
        connect!(relm, window, connect_delete_event(_, _), return (Some(Msg::Quit), Inhibit(false)));
        connect!(relm, asset_flow, connect_selected_children_changed(_), Msg::ProcessAssetSelected);

        window.show_all();

        Win {
            model,
            widgets: Widgets {
                main_stack,
                logged_in_box,
                login_box,
                window,
                login_view: webview,
                progress_message,
                asset_flow,
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
    env_logger::builder()
        .format(|buf, record| {
            writeln!(buf, "<{}> - [{}] - {}",
                     record.target(),
                     record.level(),
                     record.args())
        })
        .init();

    Win::run(()).expect("Win::run failed");
}