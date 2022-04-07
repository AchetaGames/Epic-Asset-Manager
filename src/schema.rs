table! {
    unreal_project_latest_engine (project) {
        project -> Text,
        engine -> Text,
    }
}

table! {
    favorite_asset (asset) {
        asset -> Text,
    }
}

table! {
    user_data (name) {
        name -> Text,
        value -> Text,
    }
}
