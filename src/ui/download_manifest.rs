use crate::tools::image_stock::ImageExtCust;
use crate::tools::or::Or;
use crate::Win;
use byte_unit::Byte;
use egs_api::api::types::asset_info::ReleaseInfo;
use egs_api::api::types::download_manifest::DownloadManifest;
use gtk::prelude::ComboBoxExtManual;
use gtk::{
    Align, Box, Button, ButtonExt, CheckButton, ComboBoxExt, ComboBoxTextExt, ContainerExt,
    GridBuilder, GridExt, IconSize, Label, LabelExt, ListBox, ListBoxRow, RevealerExt, StackExt,
    ToggleButtonExt, WidgetExt,
};
use relm::{connect, Channel};
use std::thread;
use tokio::runtime::Runtime;

pub(crate) trait DownloadManifests {
    fn load_download_manifest(&self, _asset_id: String, _release_info: ReleaseInfo) {
        unimplemented!()
    }

    fn process_download_manifest(
        &mut self,
        _asset_id: String,
        _download_manifest: DownloadManifest,
    ) {
        unimplemented!()
    }

    fn show_asset_download(&mut self, _enabled: bool) {
        unimplemented!()
    }

    fn download_version_selected(&mut self) {
        unimplemented!()
    }

    fn toggle_download_details(&mut self) {
        unimplemented!()
    }
}

impl DownloadManifests for Win {
    fn load_download_manifest(&self, asset_id: String, release_info: ReleaseInfo) {
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

        if let Ok(download_manifests) = crate::DATA.download_manifests.read() {
            if let Some(dm) =
                download_manifests.get(release_info.id.clone().unwrap_or(asset_id.clone()).as_str())
            {
                self.model
                    .relm
                    .stream()
                    .emit(crate::ui::messages::Msg::ProcessDownloadManifest(
                        asset_id.clone(),
                        dm.clone(),
                    ));
                return;
            }
        };

        let stream = self.model.relm.stream().clone();
        let ri = release_info.clone();
        let (_channel, sender) = Channel::new(move |dm: DownloadManifest| {
            if let Ok(mut download_manifests) = crate::DATA.download_manifests.write() {
                download_manifests.insert(ri.clone().id.unwrap_or(asset_id.clone()), dm.clone());
            }
            stream.emit(crate::ui::messages::Msg::ProcessDownloadManifest(
                asset_id.clone(),
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

    fn process_download_manifest(&mut self, id: String, dm: DownloadManifest) {
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
            self.recreate_download_buttons();
            self.widgets
                .asset_download_widgets
                .download_all
                .as_ref()
                .unwrap()
                .set_sensitive(true);
            self.widgets
                .asset_download_widgets
                .download_selected
                .as_ref()
                .unwrap()
                .set_sensitive(true);
            let i = id.clone();
            let di = dm.app_name_string.clone();
            connect!(
                self.model.relm,
                self.widgets
                    .asset_download_widgets
                    .download_selected
                    .as_ref()
                    .unwrap(),
                connect_clicked(_),
                crate::ui::messages::Msg::DownloadAssets(false, i.clone(), di.clone())
            );
            let i = id.clone();
            let di = dm.app_name_string.clone();
            connect!(
                self.model.relm,
                self.widgets
                    .asset_download_widgets
                    .download_all
                    .as_ref()
                    .unwrap(),
                connect_clicked(_),
                crate::ui::messages::Msg::DownloadAssets(true, i.clone(), di.clone())
            );
            // TODO only enable this when something is selected
            self.widgets
                .asset_download_widgets
                .asset_download_content
                .show_all();
        }
    }

    fn show_asset_download(&mut self, enabled: bool) {
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
                                        Some(release.id.as_ref().unwrap_or(&"".to_string())),
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

        self.recreate_download_buttons();

        self.widgets
            .logged_in_stack
            .set_visible_child_name(if enabled {
                "asset_download_details"
            } else {
                "main"
            });
    }
    fn download_version_selected(&mut self) {
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
                                let versions_label = Label::new(Some("Supported versions:"));
                                versions_label.set_halign(Align::Start);
                                grid.attach(&versions_label, 0, line, 1, 1);
                                let compat =
                                    Label::new(Some(&compatible.join(", ").replace("UE_", "")));
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
                                let release_date =
                                    Label::new(Some(&date.naive_local().format("%F").to_string()));
                                release_date.set_halign(Align::Start);
                                release_date.set_xalign(0.0);
                                grid.attach(&release_date, 1, line, 1, 1);
                                line += 1;
                            }
                            if let Some(ref note) = release.release_note {
                                if !note.is_empty() {
                                    let release_note_label = Label::new(Some("Release note:"));
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

    fn toggle_download_details(&mut self) {
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

impl Win {
    pub(crate) fn recreate_download_buttons(&mut self) {
        // Remove all download buttons
        self.widgets
            .asset_download_widgets
            .asset_download_actions_box
            .foreach(|el| {
                self.widgets
                    .asset_download_widgets
                    .asset_download_actions_box
                    .remove(el)
            });

        self.widgets.asset_download_widgets.download_all = Some(Button::with_label("Download All"));
        self.widgets.asset_download_widgets.download_selected =
            Some(Button::with_label("Download Selected"));
        self.widgets
            .asset_download_widgets
            .download_all
            .as_ref()
            .unwrap()
            .set_sensitive(false);
        self.widgets
            .asset_download_widgets
            .download_selected
            .as_ref()
            .unwrap()
            .set_sensitive(false);
        self.widgets
            .asset_download_widgets
            .asset_download_actions_box
            .add(
                self.widgets
                    .asset_download_widgets
                    .download_selected
                    .as_ref()
                    .unwrap(),
            );
        self.widgets
            .asset_download_widgets
            .asset_download_actions_box
            .add(
                self.widgets
                    .asset_download_widgets
                    .download_all
                    .as_ref()
                    .unwrap(),
            );
        self.widgets
            .asset_download_widgets
            .asset_download_actions_box
            .show_all();
    }
}
