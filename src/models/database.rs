use anyhow::Result;
use diesel::{prelude::*, r2d2, r2d2::ConnectionManager};
use once_cell::sync::Lazy;
use std::{fs, fs::File, path::PathBuf};

type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

static DB_PATH: Lazy<PathBuf> =
    Lazy::new(|| gtk4::glib::user_data_dir().join("epic_asset_manager"));
static POOL: Lazy<Pool> = Lazy::new(|| init_pool().expect("Failed to create a pool"));

embed_migrations!("migrations/");

pub(crate) fn connection() -> Pool {
    POOL.clone()
}

fn run_migration_on(connection: &SqliteConnection) -> Result<()> {
    info!("Running DB Migrations...");
    embedded_migrations::run_with_output(connection, &mut std::io::stdout()).map_err(From::from)
}

fn init_pool() -> Result<Pool> {
    let db_path = &DB_PATH;
    fs::create_dir_all(&db_path.to_str().unwrap())?;
    let db_path = db_path.join("eam.db");
    if !db_path.exists() {
        File::create(&db_path.to_str().unwrap())?;
    }
    let manager = ConnectionManager::<SqliteConnection>::new(db_path.to_str().unwrap());
    let pool = r2d2::Pool::builder().build(manager)?;

    {
        let db = pool.get()?;
        run_migration_on(&*db)?;
    }
    info!("Database pool initialized.");
    Ok(pool)
}
