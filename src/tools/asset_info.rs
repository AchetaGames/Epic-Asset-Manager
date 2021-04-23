use egs_api::api::types::asset_info::AssetInfo;

pub(crate) trait Search {
    fn matches_filter(&self, _tag: Option<String>, _search: Option<String>) -> bool {
        true
    }
}

impl Search for AssetInfo {
    fn matches_filter(&self, tag: Option<String>, search: Option<String>) -> bool {
        let mut tag_found = false;
        match tag {
            None => {
                tag_found = true;
            }
            Some(f) => match &self.categories {
                None => {}
                Some(categories) => {
                    for category in categories {
                        if category.path.contains(&f) {
                            tag_found = true;
                            break;
                        }
                    }
                }
            },
        }
        match search {
            None => {
                return tag_found;
            }
            Some(f) => {
                if tag_found {
                    match &self.title {
                        None => {
                            return true;
                        }
                        Some(title) => {
                            return title.to_lowercase().contains(&f.to_lowercase());
                        }
                    }
                } else {
                    return false;
                }
            }
        }
    }
}
