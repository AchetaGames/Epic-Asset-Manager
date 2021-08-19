use crate::models::row_data::RowData;
use crate::Win;
use gdk_pixbuf::traits::PixbufLoaderExt;
use gtk4::traits::{ContainerExt, ImageExt, StackExt, WidgetExt};
use gtk4::Image;

pub(crate) trait Images {
    fn process_image(&self, _asset_id: Option<String>, _image: Vec<u8>) {
        unimplemented!()
    }

    fn next_image(&self) {
        unimplemented!()
    }

    fn prev_image(&self) {
        unimplemented!()
    }
}

impl Images for Win {
    fn process_image(&self, asset_id: Option<String>, image: Vec<u8>) {
        match asset_id {
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
                    let pixbuf = pixbuf_loader.pixbuf().unwrap();
                    let width = pixbuf.width();
                    let height = pixbuf.height();

                    let max_height = self.widgets.details_revealer.allocated_height();
                    let width_percent = if max_height < 300 {
                        300.0
                    } else {
                        max_height as f64
                    } / width as f64;
                    let height_percent = 300.0 / height as f64;
                    let percent = if height_percent < width_percent {
                        height_percent
                    } else {
                        width_percent
                    };
                    let desired = (width as f64 * percent, height as f64 * percent);
                    gtkimage.set_from_pixbuf(
                        pixbuf_loader
                            .pixbuf()
                            .unwrap()
                            .scale_simple(
                                desired.0.round() as i32,
                                desired.1.round() as i32,
                                gdk_pixbuf::InterpType::Bilinear,
                            )
                            .as_ref(),
                    );
                    gtkimage.set_expand(true);
                    gtkimage.show_all();
                    self.widgets.image_stack.add(&gtkimage)
                }
            }
        }
    }

    fn next_image(&self) {
        let total = self.widgets.image_stack.children().len() as i32;
        if total > 0 {
            let current = self.widgets.image_stack.visible_child().unwrap();
            let pos = self.widgets.image_stack.child_position(&current);

            if pos + 1 >= total {
                if let Some(new) = self.widgets.image_stack.children().get(0) {
                    self.widgets.image_stack.set_visible_child(new);
                }
            } else {
                if let Some(new) = self.widgets.image_stack.children().get((pos + 1) as usize) {
                    self.widgets.image_stack.set_visible_child(new);
                }
            };
        }
    }

    fn prev_image(&self) {
        let total = self.widgets.image_stack.children().len() as i32;
        if total > 0 {
            let current = self.widgets.image_stack.visible_child().unwrap();
            let pos = self.widgets.image_stack.child_position(&current);
            if pos - 1 < 0 {
                if let Some(new) = self.widgets.image_stack.children().last() {
                    self.widgets.image_stack.set_visible_child(new);
                }
            } else {
                if let Some(new) = self.widgets.image_stack.children().get((pos - 1) as usize) {
                    self.widgets.image_stack.set_visible_child(new);
                }
            };
        }
    }
}
