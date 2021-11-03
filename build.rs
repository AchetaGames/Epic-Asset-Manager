#[cfg(target_os = "windows")]
extern crate winres;

fn main() {
    #[cfg(target_os = "windows")]
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("data/icons/io.github.achetagames.epic_asset_manager.ico");
        res.compile().unwrap();
    }
}
