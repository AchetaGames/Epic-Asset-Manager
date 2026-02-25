use egs_api::api::types::asset_info::AssetInfo;

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use egs_api::api::types::asset_info::{AssetInfo, Category};

    fn asset(title: &str, categories: Vec<&str>) -> AssetInfo {
        AssetInfo {
            title: Some(title.to_string()),
            categories: if categories.is_empty() {
                None
            } else {
                Some(
                    categories
                        .into_iter()
                        .map(|p| Category {
                            path: p.to_string(),
                        })
                        .collect(),
                )
            },
            ..Default::default()
        }
    }

    fn asset_no_title(categories: Vec<&str>) -> AssetInfo {
        AssetInfo {
            title: None,
            categories: if categories.is_empty() {
                None
            } else {
                Some(
                    categories
                        .into_iter()
                        .map(|p| Category {
                            path: p.to_string(),
                        })
                        .collect(),
                )
            },
            ..Default::default()
        }
    }

    #[test]
    fn no_filters_matches_everything() {
        let a = asset("Any Asset", vec!["some/category"]);
        assert!(a.matches_filter(None, None));
    }

    #[test]
    fn tag_matches_category_path() {
        let a = asset("MyAsset", vec!["assets/blueprints", "assets/materials"]);
        assert!(a.matches_filter(Some("blueprints".to_string()), None));
    }

    #[test]
    fn tag_no_match() {
        let a = asset("MyAsset", vec!["assets/blueprints"]);
        assert!(!a.matches_filter(Some("materials".to_string()), None));
    }

    #[test]
    fn tag_with_no_categories() {
        let a = asset("MyAsset", vec![]);
        assert!(!a.matches_filter(Some("anything".to_string()), None));
    }

    #[test]
    fn search_matches_title_case_insensitive() {
        let a = asset("Fantasy Castle Pack", vec![]);
        assert!(a.matches_filter(None, Some("fantasy".to_string())));
        assert!(a.matches_filter(None, Some("CASTLE".to_string())));
    }

    #[test]
    fn search_no_match_in_title() {
        let a = asset("Sci-Fi Corridor", vec![]);
        assert!(!a.matches_filter(None, Some("fantasy".to_string())));
    }

    #[test]
    fn search_with_no_title_matches() {
        let a = asset_no_title(vec![]);
        assert!(a.matches_filter(None, Some("anything".to_string())));
    }

    #[test]
    fn tag_and_search_both_match() {
        let a = asset("Fantasy Castle", vec!["assets/environments"]);
        assert!(a.matches_filter(Some("environments".to_string()), Some("castle".to_string())));
    }

    #[test]
    fn tag_matches_but_search_does_not() {
        let a = asset("Sci-Fi Corridor", vec!["assets/environments"]);
        assert!(!a.matches_filter(Some("environments".to_string()), Some("castle".to_string())));
    }

    #[test]
    fn tag_does_not_match_search_irrelevant() {
        let a = asset("Fantasy Castle", vec!["assets/characters"]);
        assert!(!a.matches_filter(Some("environments".to_string()), Some("castle".to_string())));
    }

    #[test]
    fn thumbnail_returns_none_without_images() {
        let a = AssetInfo::default();
        assert!(a.thumbnail().is_none());
    }

    #[test]
    fn default_trait_matches_everything() {
        struct Dummy;
        impl Search for Dummy {}
        let d = Dummy;
        assert!(d.matches_filter(Some("tag".to_string()), Some("search".to_string())));
        assert!(d.thumbnail().is_none());
    }
}
