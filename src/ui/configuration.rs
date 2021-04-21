use crate::Win;
use gio::FileExt;
use gtk::{Box, Button, ButtonExt, ContainerExt, FileChooserExt, Label, LabelExt, WidgetExt};
use relm::connect;

pub(crate) trait Configuration {
    fn configuration_directory_selection_changed(&mut self, _selector: String) {
        unimplemented!()
    }

    fn add_unreal_directory(&mut self, _selector: &str) {
        unimplemented!()
    }
    fn remove_unreal_directory(&mut self, _path: String, _selector: &str) {
        unimplemented!()
    }
    fn create_unreal_directory_widget(&mut self, _path: String, _selector: &str) {
        unimplemented!()
    }
    fn create_missing_unreal_directory_widgets(&mut self) {
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

    fn add_unreal_directory(&mut self, selector: &str) {
        if let Some(file) = self
            .widgets
            .settings_widgets
            .directory_selectors
            .get(selector)
            .unwrap()
            .file()
        {
            let path = match file.path() {
                None => return,
                Some(p) => {
                    if p.is_dir() {
                        p.display().to_string()
                    } else {
                        match p.parent() {
                            None => return,
                            Some(pa) => pa.display().to_string(),
                        }
                    }
                }
            };
            if !match selector {
                "ue_project_directory_selector" => self
                    .model
                    .configuration
                    .directories
                    .unreal_projects_directories
                    .contains(&path),
                "ue_directory_selector" => self
                    .model
                    .configuration
                    .directories
                    .unreal_engine_directories
                    .contains(&path),
                _ => true,
            } {
                match selector {
                    "ue_project_directory_selector" => self
                        .model
                        .configuration
                        .directories
                        .unreal_projects_directories
                        .push(path.clone()),
                    "ue_directory_selector" => self
                        .model
                        .configuration
                        .directories
                        .unreal_engine_directories
                        .push(path.clone()),
                    _ => {}
                };

                self.create_unreal_directory_widget(path.clone(), selector);
                self.model.configuration.save();
            } else {
                debug!("Unreal directory already in list")
            }
        }
    }

    fn create_unreal_directory_widget(&mut self, path: String, selector: &str) {
        let hbox = Box::new(gtk::Orientation::Horizontal, 0);
        let label = Label::new(Some(&path));
        hbox.add(&label);
        label.set_hexpand(true);
        label.set_xalign(0.0);
        let button = Button::with_label("-");
        let p = path.clone();
        let s = selector.to_string();
        connect!(
            self.model.relm,
            button,
            connect_clicked(_),
            crate::ui::messages::Msg::ConfigurationRemoveUnrealEngineDir(p.clone(), s.clone())
        );
        hbox.add(&button);
        hbox.set_widget_name(&path);
        match selector {
            "ue_project_directory_selector" => {
                self.widgets
                    .settings_widgets
                    .unreal_engine_project_directories_box
                    .add(&hbox);
                self.widgets
                    .settings_widgets
                    .unreal_engine_project_directories_box
                    .show_all();
            }
            "ue_directory_selector" => {
                self.widgets
                    .settings_widgets
                    .unreal_engine_directories_box
                    .add(&hbox);
                self.widgets
                    .settings_widgets
                    .unreal_engine_directories_box
                    .show_all();
            }
            _ => {}
        };
    }

    fn remove_unreal_directory(&mut self, path: String, selector: &str) {
        match selector {
            "ue_project_directory_selector" => {
                self.widgets
                    .settings_widgets
                    .unreal_engine_project_directories_box
                    .foreach(|w| {
                        if w.widget_name().eq(&path) {
                            self.widgets
                                .settings_widgets
                                .unreal_engine_project_directories_box
                                .remove(w);
                        }
                    });
                self.model
                    .configuration
                    .directories
                    .unreal_projects_directories
                    .retain(|x| !x.eq(&path));
            }
            "ue_directory_selector" => {
                self.widgets
                    .settings_widgets
                    .unreal_engine_directories_box
                    .foreach(|w| {
                        if w.widget_name().eq(&path) {
                            self.widgets
                                .settings_widgets
                                .unreal_engine_directories_box
                                .remove(w);
                        }
                    });
                self.model
                    .configuration
                    .directories
                    .unreal_engine_directories
                    .retain(|x| !x.eq(&path));
            }
            _ => {}
        };

        self.model.configuration.save();
    }

    fn create_missing_unreal_directory_widgets(&mut self) {
        let mut missing_dirs: Vec<(String, String)> = Vec::new();
        for dir in &self
            .model
            .configuration
            .directories
            .unreal_engine_directories
        {
            let mut found = false;
            for child in self
                .widgets
                .settings_widgets
                .unreal_engine_directories_box
                .children()
            {
                if child.widget_name().to_string().eq(dir) {
                    found = true;
                    break;
                }
            }
            if !found {
                missing_dirs.push(("ue_directory_selector".to_string(), dir.clone()));
            }
        }
        for dir in &self
            .model
            .configuration
            .directories
            .unreal_projects_directories
        {
            let mut found = false;
            for child in self
                .widgets
                .settings_widgets
                .unreal_engine_project_directories_box
                .children()
            {
                if child.widget_name().to_string().eq(dir) {
                    found = true;
                    break;
                }
            }
            if !found {
                missing_dirs.push(("ue_project_directory_selector".to_string(), dir.clone()));
            }
        }
        for (selector, dir) in missing_dirs {
            self.create_unreal_directory_widget(dir.clone(), &selector);
        }
    }
}
