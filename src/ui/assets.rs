use crate::tools::asset_info::Search;
use crate::Win;
use egs_api::api::types::asset_info::AssetInfo;
use gdk_pixbuf::PixbufLoaderExt;
use glib::Cast;
use gtk::{
    Align, AspectFrame, Box, Button, ButtonExt, ContainerExt, EntryExt, FlowBoxChild, FlowBoxExt,
    GridBuilder, GridExt, Image, ImageExt, Justification, Label, LabelExt, Overlay, OverlayExt,
    RevealerExt, Separator, WidgetExt,
};
use relm::connect;
use std::thread;

pub(crate) trait Assets {
    fn bind_asset_model(&self) {
        unimplemented!()
    }

    fn show_asset_details(&mut self) {
        unimplemented!()
    }

    fn close_asset_details(&mut self) {
        unimplemented!()
    }

    fn apply_asset_filter(&mut self) {
        unimplemented!()
    }

    fn search_assets(&mut self) {
        unimplemented!()
    }

    fn filter_assets(&mut self, _filter: Option<String>) {
        unimplemented!()
    }

    fn process_asset_info(&self, _asset: AssetInfo) {
        unimplemented!()
    }
}

impl Assets for Win {
    fn bind_asset_model(&self) {
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
                            .pixbuf()
                            .unwrap()
                            .scale_simple(128, 128, gdk_pixbuf::InterpType::Bilinear)
                            .as_ref(),
                    );
                }
                vbox.set_size_request(130, 150);
                vbox.add(&gtkimage);
                if let Some(title) = &data.title {
                    let label = Label::new(Some(title));
                    label.set_wrap(true);
                    label.set_expand(false);
                    label.set_max_width_chars(15);
                    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    label.set_tooltip_text(Some(title));
                    label.set_justify(Justification::Center);
                    vbox.add(&label);
                }

                vbox.set_margin(10);
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

    fn show_asset_details(&mut self) {
        for child in self.widgets.asset_flow.selected_children() {
            if let Ok(ai) = crate::DATA.asset_info.read() {
                if let Some(asset_info) = ai.get(child.widget_name().as_str()) {
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
                    let aspect = AspectFrame::new(None, 0.5, 0.5, 2.0, true);
                    aspect.set_size_request(-1, 300);
                    aspect.add(&self.widgets.image_stack);
                    image_navigation.add_overlay(&aspect);
                    image_navigation.add_overlay(&back);
                    image_navigation.add_overlay(&forward);
                    image_navigation.set_valign(Align::Center);
                    vbox.add(&image_navigation);
                    if let Some(images) = &asset_info.key_images {
                        for image in images {
                            if image.width < 300 || image.height < 300 {
                                continue;
                            }
                            self.model
                                .relm
                                .stream()
                                .emit(crate::ui::messages::Msg::DownloadImage(None, image.clone()));
                        }
                    }
                    let details_box = Box::new(gtk::Orientation::Vertical, 0);
                    details_box.set_vexpand(true);
                    details_box.set_valign(Align::Start);
                    vbox.add(&details_box);
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
                    details_box.add(&table);
                    details_box.add(&Separator::new(gtk::Orientation::Horizontal));
                    if let Some(desc) = &asset_info.long_description {
                        let description = Label::new(None);
                        description.set_line_wrap(true);
                        let markup = html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n");
                        description.set_markup(&markup);
                        description.set_xalign(0.0);
                        details_box.add(&description);
                    }
                    if let Some(desc) = &asset_info.technical_details {
                        let description = Label::new(None);
                        description.set_line_wrap(true);
                        let markup = html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n");
                        description.set_markup(&markup);
                        description.set_xalign(0.0);
                        details_box.add(&description);
                    }
                    if asset_info.release_info.clone().unwrap().len() > 0 {
                        self.widgets.download_button.set_sensitive(true);
                    }

                    self.widgets.details_content.add(&vbox);
                    self.widgets.details_content.show_all();

                    if !self.widgets.details_revealer.reveals_child() {
                        self.widgets.details_revealer.set_reveal_child(true);
                    }
                }
            }
        }
    }

    fn close_asset_details(&mut self) {
        self.widgets.download_button.set_sensitive(false);
        self.widgets.details_revealer.set_reveal_child(false);
        self.widgets
            .details_content
            .foreach(|el| self.widgets.details_content.remove(el));
        self.widgets.asset_flow.unselect_all();
    }

    fn apply_asset_filter(&mut self) {
        self.widgets
            .asset_flow
            .set_filter_func(Some(std::boxed::Box::new(|child| -> bool {
                let id = child.widget_name().to_string();
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

    fn search_assets(&mut self) {
        let search = self.widgets.search.text().to_string();
        if let Ok(mut search_filter) = crate::DATA.search_filter.write() {
            if search.is_empty() {
                *search_filter = None;
            } else {
                search_filter.replace(search.clone());
            }
        }
        self.apply_asset_filter()
    }

    fn filter_assets(&mut self, filter: Option<String>) {
        if let Ok(mut tag_filter) = crate::DATA.tag_filter.write() {
            match filter {
                None => {
                    *tag_filter = None;
                }
                Some(f) => {
                    tag_filter.replace(f);
                }
            }
        }
        self.apply_asset_filter()
    }

    fn process_asset_info(&self, asset: AssetInfo) {
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
}
