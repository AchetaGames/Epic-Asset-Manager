use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::iter::FromIterator;
use std::path::PathBuf;
use std::{fs, thread};

use byte_unit::Byte;
use egs_api::api::types::asset_info::AssetInfo;
use egs_api::api::types::download_manifest::DownloadManifest;
use egs_api::api::types::epic_asset::EpicAsset;
use egs_api::EpicGames;
use gdk_pixbuf::PixbufLoaderExt;
use glib::Cast;
use gtk::prelude::ComboBoxExtManual;
use gtk::{
    Align, Box, Button, ButtonExt, CheckButton, ComboBoxExt, ComboBoxTextExt, ContainerExt,
    EntryExt, FlowBoxChild, FlowBoxExt, GridBuilder, GridExt, IconSize, Image, ImageExt,
    Justification, Label, LabelExt, ListBox, ListBoxRow, MenuButton, MenuButtonExt, Overlay,
    OverlayExt, PopoverMenu, ProgressBarExt, RevealerExt, Separator, StackExt, ToggleButtonExt,
    WidgetExt,
};
use relm::{connect, Channel, Relm, Update};
use threadpool::ThreadPool;
use tokio::runtime::Runtime;
use webkit2gtk::{LoadEvent, WebResourceExt, WebViewExt};

use crate::configuration::Configuration;
use crate::download::DownloadedFile;
use crate::models::row_data::RowData;
use crate::tools::asset_info::Search;
use crate::tools::cache::Cache;
use crate::tools::image_stock::ImageExtCust;
use crate::tools::or::Or;
use crate::{LoginResponse, Model, Win};

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = crate::ui::messages::Msg;

    fn model(relm: &Relm<Self>, _: ()) -> Model {
        Model {
            relm: relm.clone(),
            epic_games: EpicGames::new(),
            configuration: Configuration::new(),
            asset_model: crate::models::asset_model::Model::new(),
            selected_asset: None,
            selected_files: HashMap::new(),
            download_pool: ThreadPool::new(5),
            file_pool: ThreadPool::new(5),
            downloaded_chunks: HashMap::new(),
            downloaded_files: HashMap::new(),
        }
    }

    fn update(&mut self, event: crate::ui::messages::Msg) {
        let start = std::time::Instant::now();
        match event.clone() {
            crate::ui::messages::Msg::Quit => {
                if let Ok(mut w) = crate::RUNNING.write() {
                    *w = false
                }
                gtk::main_quit()
            }
            crate::ui::messages::Msg::WebViewLoadFinished(event) => match event {
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
                                    stream.emit(crate::ui::messages::Msg::Login(sid));
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
            crate::ui::messages::Msg::Login(sid) => {
                self.widgets.progress_message.set_label("Login in progress");
                &self.widgets.main_stack.set_visible_child_name("progress");
                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |ud| {
                    if let Some(user_data) = ud {
                        stream.emit(crate::ui::messages::Msg::LoginOk(user_data));
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
            crate::ui::messages::Msg::Relogin => {
                self.widgets.progress_message.set_label("Resuming session");
                self.model
                    .epic_games
                    .set_user_details(self.model.configuration.user_data.clone().unwrap());
                &self.widgets.main_stack.set_visible_child_name("progress");
                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |ud| {
                    if let Some(user_data) = ud {
                        stream.emit(crate::ui::messages::Msg::LoginOk(user_data));
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
            crate::ui::messages::Msg::LoginOk(user_data) => {
                self.model.epic_games.set_user_details(user_data);
                self.model.configuration.user_data =
                    Some(self.model.epic_games.user_details().to_owned());
                self.model.configuration.save();
                self.widgets
                    .main_stack
                    .set_visible_child_name("logged_in_stack");

                let logout_button = Button::with_label("Logout");
                let logged_in_box = Box::new(gtk::Orientation::Vertical, 0);
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
                    stream.emit(crate::ui::messages::Msg::ProcessAssetList(anm, am));
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
            crate::ui::messages::Msg::ProcessAssetList(anm, am) => {
                // TODO: Cache EpicAsset

                if let Ok(mut asset_namespace_map) = crate::DATA.asset_namespace_map.write() {
                    asset_namespace_map.clear();
                    asset_namespace_map.extend(anm.clone())
                }
                if let Ok(mut asset_map) = crate::DATA.asset_map.write() {
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
                    stream.emit(crate::ui::messages::Msg::ProcessAssetInfo(ai));
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
                                        if let Ok(mut asset_info) = crate::DATA.asset_info.write() {
                                            asset_info.insert(asset.id.clone(), asset.clone());
                                        }
                                        s.send(asset).unwrap();
                                    };
                                }
                                Some(asset) => {
                                    if let Ok(mut asset_info) = crate::DATA.asset_info.write() {
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
            crate::ui::messages::Msg::ProcessAssetInfo(asset) => {
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
                                .emit(crate::ui::messages::Msg::DownloadImage(
                                    Some(asset.id.clone()),
                                    image.clone(),
                                ));

                            found = true;
                            break;
                        }
                    }
                    if !found {
                        debug!("{:?}: {:#?}", asset.title, asset.key_images);
                        self.model
                            .relm
                            .stream()
                            .emit(crate::ui::messages::Msg::PulseProgress);
                    }
                }
            }
            crate::ui::messages::Msg::DownloadImage(id, image) => {
                let stream = self.model.relm.stream().clone();
                let (_channel, sender) = Channel::new(move |(id, b)| {
                    stream.emit(crate::ui::messages::Msg::ProcessImage(id, b));
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
                                            let mut _data = crate::MUTEX.lock().unwrap();
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
                                let mut _data = crate::MUTEX.lock().unwrap();
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
            crate::ui::messages::Msg::ProcessImage(asset_id, image) => match asset_id {
                Some(id) => {
                    if let Ok(asset_info) = crate::DATA.asset_info.read() {
                        if let Some(asset) = asset_info.get(&id) {
                            self.model.asset_model.append(&RowData::new(
                                asset.title.clone(),
                                asset.clone(),
                                image,
                            ));
                            self.model
                                .relm
                                .stream()
                                .emit(crate::ui::messages::Msg::PulseProgress);
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
            crate::ui::messages::Msg::LoadDownloadManifest(id, release_info) => {
                let asset = match crate::DATA.asset_info.read() {
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

                if let Ok(download_manifests) = crate::DATA.download_manifests.read() {
                    if let Some(dm) = download_manifests
                        .get(release_info.id.clone().unwrap_or(id.clone()).as_str())
                    {
                        self.model.relm.stream().emit(
                            crate::ui::messages::Msg::ProcessDownloadManifest(
                                id.clone(),
                                dm.clone(),
                            ),
                        );
                        return;
                    }
                };

                let stream = self.model.relm.stream().clone();
                let ri = release_info.clone();
                let (_channel, sender) = Channel::new(move |dm: DownloadManifest| {
                    if let Ok(mut download_manifests) = crate::DATA.download_manifests.write() {
                        download_manifests.insert(ri.clone().id.unwrap_or(id.clone()), dm.clone());
                    }
                    stream.emit(crate::ui::messages::Msg::ProcessDownloadManifest(
                        id.clone(),
                        dm,
                    ));
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
            crate::ui::messages::Msg::ProcessDownloadManifest(id, dm) => {
                if self.model.selected_asset == Some(id.clone()) {
                    let size_box = Box::new(gtk::Orientation::Horizontal, 0);
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

                    let list = ListBox::new();
                    for (file, manifest) in files {
                        let row = ListBoxRow::new();
                        row.set_widget_name(&file.clone());
                        let hbox = Box::new(gtk::Orientation::Horizontal, 5);
                        let chbox = CheckButton::new();

                        let asset_id = id.clone();
                        let app_name = dm.app_name_string.clone();
                        let filename = file.clone();
                        chbox.set_active(match self.model.selected_files.get(&asset_id) {
                            None => false,
                            Some(map) => match map.get(&app_name) {
                                None => false,
                                Some(files) => files.contains(&filename),
                            },
                        });
                        connect!(
                            self.model.relm,
                            chbox,
                            connect_toggled(_),
                            crate::ui::messages::Msg::SelectForDownload(
                                asset_id.clone(),
                                app_name.clone(),
                                filename.clone()
                            )
                        );
                        hbox.add(&chbox);
                        let filename = Label::new(Some(&file.clone()));
                        filename.set_halign(Align::Fill);
                        filename.set_hexpand(true);
                        filename.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
                        filename.set_xalign(0.0);
                        hbox.add(&filename);
                        let size_label = Label::new(Some(
                            &Byte::from_bytes(
                                manifest
                                    .file_chunk_parts
                                    .iter()
                                    .map(|chunk| chunk.size)
                                    .sum(),
                            )
                            .get_appropriate_unit(false)
                            .to_string(),
                        ));
                        size_label.set_size_request(50, -1);
                        size_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                        size_label.set_xalign(1.0);

                        hbox.add(&size_label);

                        row.add(&hbox);
                        list.add(&row);
                    }
                    let scrolled_window =
                        gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
                    scrolled_window.add(&list);
                    scrolled_window.set_vexpand(true);
                    scrolled_window.set_hexpand(true);
                    self.widgets
                        .asset_download_widgets
                        .asset_download_content
                        .add(&scrolled_window);
                    self.widgets
                        .asset_download_widgets
                        .download_all
                        .set_sensitive(true);
                    let i = id.clone();
                    let di = dm.app_name_string.clone();
                    connect!(
                        self.model.relm,
                        self.widgets.asset_download_widgets.download_selected,
                        connect_clicked(_),
                        crate::ui::messages::Msg::DownloadAssets(false, i.clone(), di.clone())
                    );
                    self.widgets
                        .asset_download_widgets
                        .download_all
                        .set_sensitive(true);
                    let i = id.clone();
                    let di = dm.app_name_string.clone();
                    connect!(
                        self.model.relm,
                        self.widgets.asset_download_widgets.download_all,
                        connect_clicked(_),
                        crate::ui::messages::Msg::DownloadAssets(true, i.clone(), di.clone())
                    );
                    // TODO only enable this when something is selected
                    self.widgets
                        .asset_download_widgets
                        .download_selected
                        .set_sensitive(true);
                    self.widgets
                        .asset_download_widgets
                        .asset_download_content
                        .show_all();
                }
            }
            crate::ui::messages::Msg::ProcessAssetSelected => {
                for child in self.widgets.asset_flow.get_selected_children() {
                    if let Ok(ai) = crate::DATA.asset_info.read() {
                        if let Some(asset_info) = ai.get(child.get_widget_name().as_str()) {
                            self.widgets
                                .details_content
                                .foreach(|el| self.widgets.details_content.remove(el));

                            info!("Showing details for {:?}", asset_info.title);
                            self.model.selected_asset = Some(asset_info.id.clone());

                            let vbox = Box::new(gtk::Orientation::Vertical, 0);

                            if let Some(title) = &asset_info.title {
                                let name = Label::new(None);
                                name.set_markup(&format!("<b><u><big>{}</big></u></b>", title));
                                name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
                                name.set_line_wrap(true);
                                name.set_halign(Align::Start);
                                vbox.add(&name);
                            }

                            vbox.add(&Separator::new(gtk::Orientation::Horizontal));
                            self.widgets
                                .image_stack
                                .foreach(|el| self.widgets.image_stack.remove(el));
                            let image_navigation = Overlay::new();
                            image_navigation.set_size_request(-1, 300);
                            let back = Button::with_label("<");
                            back.set_halign(Align::Start);
                            back.set_opacity(0.5);
                            connect!(
                                self.model.relm,
                                back,
                                connect_clicked(_),
                                crate::ui::messages::Msg::PrevImage
                            );
                            let forward = Button::with_label(">");
                            forward.set_opacity(0.5);
                            connect!(
                                self.model.relm,
                                forward,
                                connect_clicked(_),
                                crate::ui::messages::Msg::NextImage
                            );
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
                                    self.model.relm.stream().emit(
                                        crate::ui::messages::Msg::DownloadImage(
                                            None,
                                            image.clone(),
                                        ),
                                    );
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
                                developer_name.set_xalign(0.0);
                                table.attach(&developer_name, 1, 0, 1, 1);
                            }
                            let platforms_label = Label::new(Some("Platforms:"));
                            platforms_label.set_halign(Align::Start);
                            table.attach(&platforms_label, 0, 1, 1, 1);
                            let platforms = Label::new(Some(
                                &asset_info.get_platforms().unwrap_or_default().join(", "),
                            ));
                            platforms.set_halign(Align::Start);
                            platforms.set_xalign(0.0);
                            platforms.set_line_wrap(true);
                            table.attach(&platforms, 1, 1, 1, 1);
                            let comp_label = Label::new(Some("Compatible with:"));
                            comp_label.set_halign(Align::Start);

                            table.attach(&comp_label, 0, 2, 1, 1);
                            if let Some(comp) = &asset_info.get_compatible_apps() {
                                let compat = Label::new(Some(&comp.join(", ").replace("UE_", "")));
                                compat.set_halign(Align::Start);
                                compat.set_line_wrap(true);
                                compat.set_xalign(0.0);
                                table.attach(&compat, 1, 2, 1, 1);
                            }
                            vbox.add(&table);
                            vbox.add(&Separator::new(gtk::Orientation::Horizontal));
                            if let Some(desc) = &asset_info.long_description {
                                let description = Label::new(None);
                                description.set_line_wrap(true);
                                let markup =
                                    html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n");
                                description.set_markup(&markup);
                                description.set_xalign(0.0);
                                vbox.add(&description);
                            }
                            if let Some(desc) = &asset_info.technical_details {
                                let description = Label::new(None);
                                description.set_line_wrap(true);
                                let markup =
                                    html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n");
                                description.set_markup(&markup);
                                description.set_xalign(0.0);
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
            crate::ui::messages::Msg::FilterNone => {
                if let Ok(mut tag_filter) = crate::DATA.tag_filter.write() {
                    *tag_filter = None;
                }
                self.model
                    .relm
                    .stream()
                    .emit(crate::ui::messages::Msg::ApplyFilter);
            }
            crate::ui::messages::Msg::FilterSome(filter) => {
                if let Ok(mut tag_filter) = crate::DATA.tag_filter.write() {
                    tag_filter.replace(filter);
                }
                self.model
                    .relm
                    .stream()
                    .emit(crate::ui::messages::Msg::ApplyFilter);
            }
            crate::ui::messages::Msg::Search => {
                let search = self.widgets.search.get_text().to_string();
                if let Ok(mut search_filter) = crate::DATA.search_filter.write() {
                    if search.is_empty() {
                        *search_filter = None;
                    } else {
                        search_filter.replace(search.clone());
                    }
                }
                self.model
                    .relm
                    .stream()
                    .emit(crate::ui::messages::Msg::ApplyFilter);
            }
            crate::ui::messages::Msg::ApplyFilter => {
                self.widgets
                    .asset_flow
                    .set_filter_func(Some(std::boxed::Box::new(|child| -> bool {
                        let id = child.get_widget_name().to_string();
                        match crate::DATA.asset_info.read() {
                            Ok(asset_info) => match asset_info.get(&id) {
                                Some(ai) => {
                                    let tag = match crate::DATA.tag_filter.read() {
                                        Ok(tag_filter) => tag_filter.clone(),
                                        Err(_) => None,
                                    };
                                    let search = match crate::DATA.search_filter.read() {
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
            crate::ui::messages::Msg::BindAssetModel => {
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
                        let vbox = Box::new(gtk::Orientation::Vertical, 0);
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
            crate::ui::messages::Msg::PulseProgress => {
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
            crate::ui::messages::Msg::CloseDetails => {
                self.widgets.download_button.set_sensitive(false);
                self.widgets.details_revealer.set_reveal_child(false);
                self.widgets.asset_flow.unselect_all();
            }
            crate::ui::messages::Msg::NextImage => {
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
            crate::ui::messages::Msg::PrevImage => {
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
            crate::ui::messages::Msg::ShowSettings(enabled) => {
                self.widgets
                    .logged_in_stack
                    .set_visible_child_name(if enabled { "settings" } else { "main" });
            }
            crate::ui::messages::Msg::ShowAssetDownload(enabled) => {
                // Cleanup
                self.widgets
                    .asset_download_widgets
                    .asset_version_combo
                    .remove_all();

                if enabled {
                    if let Some(asset_id) = &self.model.selected_asset {
                        if let Ok(ai) = crate::DATA.asset_info.read() {
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
                    .asset_download_widgets
                    .download_all
                    .set_sensitive(false);
                self.widgets
                    .asset_download_widgets
                    .download_selected
                    .set_sensitive(false);

                self.widgets
                    .logged_in_stack
                    .set_visible_child_name(if enabled {
                        "asset_download_details"
                    } else {
                        "main"
                    });
            }
            crate::ui::messages::Msg::DownloadVersionSelected => {
                if let Some(id) = self
                    .widgets
                    .asset_download_widgets
                    .asset_version_combo
                    .get_active_id()
                {
                    if let Some(asset_id) = &self.model.selected_asset {
                        if let Ok(ai) = crate::DATA.asset_info.read() {
                            if let Some(asset_info) = ai.get(asset_id) {
                                // Show Download details if loading new manifest
                                if !self
                                    .widgets
                                    .asset_download_widgets
                                    .asset_download_info_revealer
                                    .get_reveal_child()
                                {
                                    self.model
                                        .relm
                                        .stream()
                                        .emit(crate::ui::messages::Msg::ToggleAssetDownloadDetails)
                                }
                                // Clear download info
                                self.widgets
                                    .asset_download_widgets
                                    .asset_download_info_box
                                    .foreach(|el| {
                                        self.widgets
                                            .asset_download_widgets
                                            .asset_download_info_box
                                            .remove(el)
                                    });
                                // Clear Download Content
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
                                if let Some(release) = asset_info.get_release_id(&id.to_string()) {
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
                                        compat.set_xalign(0.0);
                                        grid.attach(&compat, 1, line, 1, 1);
                                        line += 1;
                                    }
                                    if let Some(ref platforms) = release.platform {
                                        let platforms_label = Label::new(Some("Platforms:"));
                                        platforms_label.set_halign(Align::Start);
                                        grid.attach(&platforms_label, 0, line, 1, 1);
                                        let platforms = Label::new(Some(&platforms.join(", ")));
                                        platforms.set_halign(Align::Start);
                                        platforms.set_xalign(0.0);
                                        platforms.set_line_wrap(true);
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
                                        release_date.set_xalign(0.0);
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
                                    self.model.relm.stream().emit(
                                        crate::ui::messages::Msg::LoadDownloadManifest(
                                            asset_info.id.clone(),
                                            release,
                                        ),
                                    );
                                };
                            }
                        }
                    }
                }
            }
            crate::ui::messages::Msg::ToggleAssetDownloadDetails => {
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
            crate::ui::messages::Msg::SelectForDownload(asset_id, app_name, filename) => {
                match self.model.selected_files.get_mut(&asset_id) {
                    None => {
                        self.model.selected_files.insert(
                            asset_id,
                            HashMap::from_iter(
                                [(app_name, vec![filename])]
                                    .iter()
                                    .cloned()
                                    .collect::<HashMap<String, Vec<String>>>(),
                            ),
                        );
                    }
                    Some(map) => match map.get_mut(&app_name) {
                        None => {
                            map.insert(app_name, vec![filename]);
                        }
                        Some(files) => {
                            match files.iter().position(|r| r.eq(&filename)) {
                                None => {
                                    files.push(filename);
                                }
                                Some(i) => {
                                    files.remove(i);
                                }
                            };
                        }
                    },
                };
            }
            crate::ui::messages::Msg::DownloadAssets(all, asset_id, release) => {
                println!(
                    "Starting download for {} release {}",
                    asset_id.clone(),
                    release.clone()
                );
                let asset = match crate::DATA.asset_info.read() {
                    Ok(asset_map) => match asset_map.get(asset_id.as_str()) {
                        None => {
                            return;
                        }
                        Some(a) => a.clone(),
                    },
                    Err(_) => {
                        return;
                    }
                };

                let rel = match asset.get_release_name(&release) {
                    None => {
                        return;
                    }
                    Some(rel) => rel,
                };

                if let Ok(download_manifests) = crate::DATA.download_manifests.read() {
                    if let Some(dm) =
                        download_manifests.get(rel.id.clone().unwrap_or(asset_id.clone()).as_str())
                    {
                        let mut chunks: HashSet<String> = HashSet::new();
                        let mut path = PathBuf::from(
                            self.model
                                .configuration
                                .directories
                                .temporary_download_directory
                                .clone(),
                        );
                        path.push(asset.id.clone());
                        path.push(release.clone());
                        let path = path.clone();
                        fs::create_dir_all(path.clone()).unwrap();
                        let files = if !all {
                            if let Some(map) = self.model.selected_files.get(&asset.id) {
                                if let Some(files) = map.get(&release) {
                                    Some(files)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        for (filename, manifest) in dm.get_files() {
                            if let Some(file_list) = files {
                                if !file_list.contains(&filename) {
                                    continue;
                                }
                            }
                            let downloaded = DownloadedFile {
                                asset: asset.id.clone(),
                                release: release.clone(),
                                name: filename.clone(),
                                chunks: manifest.file_chunk_parts.clone(),
                                finished_chunks: vec![],
                            };
                            let full_filename = format!(
                                "{}/{}/{}",
                                asset.id.clone(),
                                release.clone(),
                                filename.clone()
                            );
                            self.model
                                .downloaded_files
                                .insert(full_filename.clone(), downloaded);
                            for chunk in manifest.file_chunk_parts {
                                match self.model.downloaded_chunks.get_mut(&chunk.guid) {
                                    None => {
                                        self.model.downloaded_chunks.insert(
                                            chunk.guid.clone(),
                                            vec![full_filename.clone()],
                                        );
                                    }
                                    Some(files) => files.push(full_filename.clone()),
                                }
                                if !chunks.contains(&chunk.guid) {
                                    let link = chunk.link.unwrap();
                                    let mut p = path.clone();
                                    let g = chunk.guid.clone();
                                    p.push(format!("{}.chunk", g));
                                    let sender = self
                                        .widgets
                                        .asset_download_widgets
                                        .download_progress_sender
                                        .clone();
                                    self.model.download_pool.execute(move || {
                                        println!(
                                            "Downloading chunk {} from {} to {:?}",
                                            g,
                                            link.to_string(),
                                            p
                                        );
                                        let mut client = reqwest::blocking::get(link).unwrap();
                                        let mut buffer: [u8; 1024] = [0; 1024];
                                        let mut downloaded: u128 = 0;
                                        let mut file = File::create(p).unwrap();
                                        loop {
                                            match client.read(&mut buffer) {
                                                Ok(size) => {
                                                    if size > 0 {
                                                        downloaded += size as u128;
                                                        file.write(&buffer[0..size]).unwrap();
                                                        sender
                                                            .send((g.clone(), size as u128, false))
                                                            .unwrap();
                                                    } else {
                                                        break;
                                                    }
                                                }
                                                Err(e) => {
                                                    error!("Download error: {:?}", e);
                                                    break;
                                                }
                                            }
                                        }
                                        sender.send((g.clone(), downloaded.clone(), true)).unwrap();
                                    });

                                    chunks.insert(chunk.guid.clone());
                                }
                            }
                        }

                        return;
                    }
                };
            }
            crate::ui::messages::Msg::DownloadProgressReport(guid, progress, finished) => {
                if finished {
                    debug!("Finished downloading {}", guid);
                    if let Some(files) = self.model.downloaded_chunks.get(&guid) {
                        for file in files {
                            debug!("Affected files: {}", file);
                            if let Some(f) = self.model.downloaded_files.get_mut(file) {
                                for chunk in &f.chunks {
                                    if chunk.guid == guid {
                                        f.finished_chunks.push(chunk.clone());
                                        break;
                                    }
                                }
                                if f.finished_chunks.len() == f.chunks.len() {
                                    debug!("File finished {}", f.name);
                                    let finished = f.clone();
                                    let mut path = PathBuf::from(
                                        self.model
                                            .configuration
                                            .directories
                                            .unreal_vault_directory
                                            .clone(),
                                    );
                                    let mut temp_path = PathBuf::from(
                                        self.model
                                            .configuration
                                            .directories
                                            .temporary_download_directory
                                            .clone(),
                                    );
                                    temp_path.push(finished.asset.clone());
                                    temp_path.push(finished.release.clone());
                                    self.model.file_pool.execute(move || {
                                        path.push(finished.asset);
                                        path.push(finished.release);
                                        path.push(finished.name);
                                        fs::create_dir_all(path.parent().unwrap().clone()).unwrap();
                                        match fs::OpenOptions::new().append(true).create(true).open(path.clone())
                                        {
                                            Ok(mut target) => {
                                                for chunk in finished.chunks {
                                                    let mut t = temp_path.clone();
                                                    t.push(format!("{}.chunk", chunk.guid));
                                                    match File::open(t) {
                                                        Ok(mut f) => {
                                                            let metadata =  f.metadata().expect("Unable to read metadata");
                                                            let mut buffer =
                                                                vec![0 as u8; metadata.len() as usize];
                                                            f.read(&mut buffer).expect("Read failed");
                                                            let ch =
                                                                egs_api::api::types::chunk::Chunk::from_vec(
                                                                    buffer,
                                                                ).unwrap();
                                                            if (ch.uncompressed_size.unwrap_or(ch.data.len() as u32) as u128) < chunk.offset+ chunk.size {
                                                               println!("Chunk is not big enough");
                                                                break;
                                                            };
                                                            target.write(&ch.data[chunk.offset as usize..(chunk.offset+ chunk.size) as usize]).unwrap();
                                                        }
                                                        Err(e) => {
                                                            println!("Error opening the chunk file: {:?}", e)
                                                        }
                                                    }
                                                    println!("chunk: {:?}", chunk);
                                                }
                                            }
                                            Err(e) => {
                                                println!("Error opening the target file: {:?}", e)
                                            }
                                        }
                                    })
                                }
                            }
                        }
                    }
                } else {
                    debug!("Got progress report from {}, current: {}", guid, progress);
                }
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
