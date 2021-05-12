use crate::application::EpicAssetManager;
use crate::config::{APP_ID, PROFILE};
use glib::clone;
use glib::signal::Inhibit;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{gio, glib, CompositeTemplate};
use gtk_macros::action;
use log::warn;

mod imp;

glib::wrapper! {
    pub struct EpicAssetManagerWindow(ObjectSubclass<imp::EpicAssetManagerWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, gio::ActionMap, gio::ActionGroup;
}

impl EpicAssetManagerWindow {
    pub fn new(app: &EpicAssetManager) -> Self {
        let window: Self = glib::Object::new(&[]).expect("Failed to create EpicAssetManagerWindow");
        window.set_application(Some(app));
        // TODO: Set subwidget things here
        // Set icons for shell
        gtk::Window::set_default_icon_name(APP_ID);
        let self_ = imp::EpicAssetManagerWindow::from_instance(&window);
        self_.sid_box.set_window(&window);
        println!("set window");

        window
    }

    pub fn data(&self) -> &imp::EpicAssetManagerWindow {
        imp::EpicAssetManagerWindow::from_instance(self)
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let settings = &imp::EpicAssetManagerWindow::from_instance(self).settings;

        let size = self.default_size();

        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;

        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let settings = &imp::EpicAssetManagerWindow::from_instance(self).settings;

        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let is_maximized = settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    pub fn setup_actions(&self) {
        action!(
            self,
            "login",
            Some(&String::static_variant_type()),
            clone!(@weak self as window => move |_, sid| {
            println!("login called: {:?}", sid.unwrap().str());
                        })
        );
    }
}
