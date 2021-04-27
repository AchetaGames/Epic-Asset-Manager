use egs_api::EpicGames;
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
use gtk::traits::{EntryExt, ProgressBarExt, RevealerExt, StackExt, WidgetExt};
use gtk::Application;
use slab_tree::TreeBuilder;

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = Application;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(relm: &Relm<Self>, app: Application) -> Model {
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
            download_manifest_tree: TreeBuilder::new().with_root(None).build(),
            download_manifest_handlers: HashMap::new(),
            download_manifest_file_details: HashMap::new(),
            selected_files_size: 0,
            application: app,
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
                    self.widgets.loading_progress.fraction()
                        + self.widgets.loading_progress.pulse_step(),
                );
                if (self.widgets.loading_progress.fraction() * 10000.0).round() / 10000.0 == 1.0 {
                    debug!("Hiding progress");
                    self.widgets.progress_revealer.set_reveal_child(false);
                }
            }
            Msg::CloseDetails => self.close_asset_details(),
            Msg::NextImage => self.next_image(),
            Msg::PrevImage => self.prev_image(),
            Msg::ShowSettings(enabled) => {
                crate::ui::configuration::Configuration::create_missing_unreal_directory_widgets(
                    self,
                );
                self.widgets
                    .logged_in_stack
                    .set_visible_child_name(if enabled { "settings" } else { "main" });
            }
            Msg::ShowAssetDownload(enabled) => self.show_asset_download(enabled),
            Msg::DownloadVersionSelected => self.download_version_selected(),
            Msg::ToggleAssetDownloadDetails => self.toggle_download_details(),
            Msg::SelectForDownload(asset_id, app_name, filename, chbox_id, size) => {
                self.select_file_for_download(asset_id, app_name, filename, chbox_id, size)
            }
            Msg::DownloadAssets(all, asset_id, release) => {
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
            Msg::DownloadFileValidated(asset_id, release, filename, manifest) => {
                self.download_file_validated(asset_id, release, filename, manifest)
            }
            Msg::ConfigurationAddUnrealEngineDir(selector) => {
                crate::ui::configuration::Configuration::add_unreal_directory(self, &selector)
            }
            Msg::ConfigurationRemoveUnrealEngineDir(path, selector) => {
                crate::ui::configuration::Configuration::remove_unreal_directory(
                    self, path, &selector,
                )
            }
            Msg::PasswordLogin => {}
            Msg::AlternateLogin => {
                self.widgets.login_widgets.sid_entry.set_text("");
                self.widgets.main_stack.set_visible_child_name("sid_box");},
            Msg::OpenBrowserSid => {
                if let Err(_) = gio::AppInfo::launch_default_for_uri("https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect", None::<&gio::AppLaunchContext>) {
                    error!("Please go to https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect")
                }
            }
            Msg::SidLogin => {
                let sid=self.widgets.login_widgets.sid_entry.text().to_string();
                self.model.relm.stream().emit(crate::ui::messages::Msg::Login(sid));
            }
            Msg::Open(f, s) => {
                println!("{:?}", f);
                println!("{}", s);
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
