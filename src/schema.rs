diesel::table! {
    unreal_project_latest_engine (project) {
        project -> Text,
        engine -> Text,
    }
}

diesel::table! {
    favorite_asset (asset) {
        asset -> Text,
    }
}

diesel::table! {
    user_data (name) {
        name -> Text,
        value -> Text,
    }
}
