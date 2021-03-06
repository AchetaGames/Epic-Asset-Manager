use crate::configuration::Save;
use crate::tools::cache::Cache;
use crate::Win;
use egs_api::api::types::epic_asset::EpicAsset;
use egs_api::api::UserData;
use gtk::traits::{
    ButtonExt, ContainerExt, EntryExt, LabelExt, MenuButtonExt, StackExt, WidgetExt,
};
use gtk::{Box, Button, MenuButton, PopoverMenu};
use relm::{connect, Channel};
use std::collections::HashMap;
use std::thread;
use tokio::runtime::Runtime;

pub(crate) trait Authorization {
    fn show_login(&self) {
        unimplemented!()
    }
    fn login(&self, _sid: String) {
        unimplemented!()
    }

    fn relogin(&mut self) {
        unimplemented!()
    }

    fn login_ok(&mut self, _user_data: UserData) {
        unimplemented!()
    }

    fn logout(&mut self) {
        unimplemented!()
    }
}

impl Authorization for Win {
    fn show_login(&self) {
        self.widgets
            .title_right_box
            .foreach(|el| self.widgets.title_right_box.remove(el));
        self.widgets.login_widgets.login_entry.set_text("");
        self.widgets.login_widgets.password_entry.set_text("");
        self.widgets.main_stack.set_visible_child_name("sid_box");
        if let Some(ud) = &self.model.configuration.user_data {
            ud.remove(self.model.configuration.path.clone());
        };
        self.model
            .relm
            .stream()
            .emit(crate::ui::messages::Msg::AlternateLogin);
    }

    fn login(&self, sid: String) {
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
            if let Some(exchange_token) = Runtime::new().unwrap().block_on(eg.auth_sid(s.as_str()))
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

    fn relogin(&mut self) {
        println!("Starting relogin");
        self.widgets.progress_message.set_label("Resuming session");
        println!("Changed the message");
        self.model
            .epic_games
            .set_user_details(self.model.configuration.user_data.clone().unwrap());
        &self.widgets.main_stack.set_visible_child_name("progress");
        println!("Shown progress");
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

    fn login_ok(&mut self, user_data: UserData) {
        self.model.epic_games.set_user_details(user_data);
        self.model.configuration.user_data = Some(self.model.epic_games.user_details().to_owned());
        self.model.configuration.save();
        self.widgets
            .main_stack
            .set_visible_child_name("logged_in_stack");

        self.create_logout_menu();

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
                        asset_namespace_map
                            .insert(asset.namespace.clone(), vec![asset.catalog_item_id.clone()]);
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

    fn logout(&mut self) {
        self.model.epic_games.set_user_details(UserData::new());
        self.widgets
            .title_right_box
            .foreach(|el| self.widgets.details_content.remove(el));
        let stream = self.model.relm.stream().clone();
        let (_channel, sender) = Channel::new(move |_| {
            stream.emit(crate::ui::messages::Msg::ShowLogin);
        });

        let mut eg = self.model.epic_games.clone();
        thread::spawn(move || {
            let start = std::time::Instant::now();
            Runtime::new().unwrap().block_on(eg.logout());
            sender.send(true).unwrap();
            debug!(
                "{:?} - Logout requests took {:?}",
                thread::current().id(),
                start.elapsed()
            );
        });
    }
}

impl Win {
    fn create_logout_menu(&mut self) {
        let logout_button = Button::with_label("Logout");
        connect!(
            self.model.relm,
            logout_button,
            connect_clicked(_),
            crate::ui::messages::Msg::Logout
        );
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
        login_name.set_popover(Some(&logout_menu));

        &self.widgets.title_right_box.add(&login_name);
        &self.widgets.title_right_box.show_all();
    }
}
