use crate::ui::messages::Msg;
use crate::window::EpicAssetManagerWindow;
use log::debug;
use std::thread;

pub(crate) trait Update {
    fn update(&self, _event: Msg) {
        unimplemented!()
    }
}

impl Update for EpicAssetManagerWindow {
    fn update(&self, event: Msg) {
        let start = std::time::Instant::now();
        let self_: &crate::window::imp::EpicAssetManagerWindow = (*self).data();

        match event.clone() {
            Msg::ShowLogin => self.show_login(),
            Msg::LoginOk(ud) => self.show_assets(ud),
            Msg::ProcessAssetInfo(a) => {
                self_.logged_in_stack.load_thumbnail(&a);
            }
            Msg::ProcessAssetThumbnail(a, i) => {
                self_.logged_in_stack.add_asset(&a, i.as_slice());
            }
            Msg::DownloadImage(image, asset) => {
                self_.download_manager.download_thumbnail(
                    image,
                    asset,
                    self_.model.borrow().sender.clone(),
                );
            }
            Msg::FlushAssetThumbnails => {
                self_.logged_in_stack.flush_assets();
            }
            Msg::ProcessEpicAsset(epic_asset) => {
                self_.logged_in_stack.process_epic_asset(&epic_asset);
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
