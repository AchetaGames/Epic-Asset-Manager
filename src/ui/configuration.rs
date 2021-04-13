use crate::Win;
use gio::FileExt;
use gtk::FileChooserExt;

pub(crate) trait Configuration {
    fn configuration_directory_selection_changed(&mut self, _selector: String) {
        unimplemented!()
    }
}

impl Configuration for Win {
    fn configuration_directory_selection_changed(&mut self, selector: String) {
        debug!("Selection changed on {}", selector);
        if let Some(file) = self
            .widgets
            .settings_widgets
            .directory_selectors
            .get(&selector)
            .unwrap()
            .file()
        {
            if let Some(path) = file.path() {
                match selector.as_str() {
                    "ue_asset_vault_directory_selector" => {
                        self.model.configuration.directories.unreal_vault_directory =
                            path.as_path().display().to_string();
                        self.model.configuration.save();
                    }
                    "cache_directory_selector" => {
                        self.model.configuration.directories.cache_directory =
                            path.as_path().display().to_string();
                        self.model.configuration.save();
                    }
                    "temp_download_directory_selector" => {
                        self.model
                            .configuration
                            .directories
                            .temporary_download_directory = path.as_path().display().to_string();
                        self.model.configuration.save();
                    }
                    _ => {}
                }
            }
        }
    }
}
