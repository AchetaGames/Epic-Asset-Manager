use egs_api::api::types::asset_info::AssetInfo;

pub trait Search {
    fn matches_filter(&self, _tag: Option<String>, _search: Option<String>) -> bool {
        true
    }
    fn thumbnail(&self) -> Option<egs_api::api::types::asset_info::KeyImage> {
        None
    }
}

impl Search for AssetInfo {
    fn thumbnail(&self) -> Option<egs_api::api::types::asset_info::KeyImage> {
        if let Some(images) = self.key_images.clone() {
            for image in images {
                let t = image.type_field.to_lowercase();
                if t.eq_ignore_ascii_case("Thumbnail") || t.eq_ignore_ascii_case("DieselGameBox") {
                    return Some(image);
                }
            }
        };
        None
    }

    fn matches_filter(&self, tag: Option<String>, search: Option<String>) -> bool {
        let mut tag_found = false;
        match tag {
            None => {
                tag_found = true;
            }
            Some(f) => {
                if let Some(categories) = &self.categories {
                    for category in categories {
                        if category.path.contains(&f) {
                            tag_found = true;
                            break;
                        }
                    }
                }
            }
        }
        search.map_or(tag_found, |f| {
            if tag_found {
                self.title.as_ref().map_or(true, |title| {
                    title.to_lowercase().contains(&f.to_lowercase())
                })
            } else {
                false
            }
        })
    }
}
