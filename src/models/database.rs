use anyhow::Result;
use diesel::{prelude::*, r2d2, r2d2::ConnectionManager};
use diesel_migrations::MigrationHarness;
use log::info;
use once_cell::sync::Lazy;
use std::{error::Error, fs, fs::File, path::PathBuf};

type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

static DB_PATH: Lazy<PathBuf> =
    Lazy::new(|| gtk4::glib::user_data_dir().join("epic_asset_manager"));
static POOL: Lazy<Pool> = Lazy::new(|| init_pool().expect("Failed to create a pool"));

pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    diesel_migrations::embed_migrations!("migrations/");

pub fn connection() -> Pool {
    POOL.clone()
}

fn run_migration_on(
    connection: &mut SqliteConnection,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    info!("Running DB Migrations...");
    connection.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}

fn init_pool() -> Result<Pool, Box<dyn Error + Send + Sync + 'static>> {
    let db_path = &DB_PATH;
    fs::create_dir_all(&db_path.to_str().unwrap())?;
    let db_path = db_path.join("eam.db");
    if !db_path.exists() {
        File::create(&db_path.to_str().unwrap())?;
    }
    let manager = ConnectionManager::<SqliteConnection>::new(db_path.to_str().unwrap());
    let pool = r2d2::Pool::builder().build(manager)?;

    {
        let mut db = pool.get()?;
        run_migration_on(&mut db)?;
    }
    log::info!("Database pool initialized.");
    Ok(pool)
}
