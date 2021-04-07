use egs_api::EpicGames;
use gtk::{ProgressBarExt, RevealerExt, StackExt};
use relm::{Relm, Update};
use std::collections::HashMap;
use std::thread;
use threadpool::ThreadPool;

use crate::configuration::Configuration;
use crate::download::chunks::Chunks;
use crate::ui::assets::Assets;
use crate::ui::authentication::Authorization;
use crate::ui::download_manifest::DownloadManifests;
use crate::ui::epic_assets::EpicAssets;
use crate::ui::images::Images;
use crate::ui::messages::Msg;
use crate::{Model, Win};

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
            selected_files: HashMap::new(),
            download_pool: ThreadPool::with_name("Download Pool".to_string(), 5),
            thumbnail_pool: ThreadPool::with_name("Thumbnail Pool".to_string(), 5),
            image_pool: ThreadPool::with_name("Image Pool".to_string(), 5),
            file_pool: ThreadPool::with_name("File Pool".to_string(), 5),
            downloaded_chunks: HashMap::new(),
            downloaded_files: HashMap::new(),
        }
    }

    fn update(&mut self, event: Msg) {
        let start = std::time::Instant::now();
        match event.clone() {
            Msg::Quit => {
                if let Ok(mut w) = crate::RUNNING.write() {
                    *w = false
                }
                gtk::main_quit()
            }
            Msg::WebViewLoadFinished(event) => self.web_view_manage(event),
            Msg::Login(sid) => self.login(sid),
            Msg::Relogin => self.relogin(),
            Msg::LoginOk(user_data) => self.login_ok(user_data),
            Msg::ProcessAssetList(anm, am) => self.process_list(anm, am),
            Msg::ProcessAssetInfo(asset) => self.process_asset_info(asset),
            Msg::DownloadImage(id, image) => {
                crate::download::images::Images::download_image(self, id, image)
            }
            Msg::ProcessImage(asset_id, image) => {
                crate::ui::images::Images::process_image(self, asset_id, image)
            }
            Msg::LoadDownloadManifest(id, release_info) => {
                self.load_download_manifest(id, release_info)
            }
            Msg::ProcessDownloadManifest(id, dm) => self.process_download_manifest(id, dm),
            Msg::ProcessAssetSelected => self.show_asset_details(),
            Msg::FilterAssets(filter) => self.filter_assets(filter),
            Msg::SearchAssets => self.search_assets(),
            Msg::BindAssetModel => self.bind_asset_model(),
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
            Msg::CloseDetails => self.close_asset_details(),
            Msg::NextImage => self.next_image(),
            Msg::PrevImage => self.prev_image(),
            Msg::ShowSettings(enabled) => {
                self.widgets
                    .logged_in_stack
                    .set_visible_child_name(if enabled { "settings" } else { "main" });
            }
            Msg::ShowAssetDownload(enabled) => self.show_asset_download(enabled),
            Msg::DownloadVersionSelected => self.download_version_selected(),
            Msg::ToggleAssetDownloadDetails => self.toggle_download_details(),
            Msg::SelectForDownload(asset_id, app_name, filename) => {
                self.select_file_for_download(asset_id, app_name, filename)
            }
            Msg::DownloadAssets(all, asset_id, release) => {
                self.chunk_init_download(all, asset_id, release)
            }
            Msg::DownloadProgressReport(guid, progress, finished) => {
                self.chunk_download_progress_report(guid, progress, finished)
            }
            Msg::ExtractionFinished(file, path) => self.chunk_extraction_finished(file, path),
            Msg::ConfigurationDirectorySelectionChanged(selector) => {
                crate::ui::configuration::Configuration::configuration_directory_selection_changed(
                    self, selector,
                )
            }
            Msg::Logout => self.logout(),
            Msg::ShowLogin => self.show_login(),
        }
        debug!(
            "{:?} - {} took {:?}",
            thread::current().id(),
            event,
            start.elapsed()
        );
    }
}
