use rusqlite::Connection;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use tokio::runtime::Builder;

async fn init_db() {

    let home_fallback = dirs::home_dir()
        .map(|d| d.join(".rg/sqlx/data_store.sqlite"))
        .and_then(|d| d.to_str().map(|s| s.to_string()))
        .map(|d| format!("sqlite://{}", d));

    let path = std::env::var("DATABASE_URL")
        .ok().or(home_fallback)
        .expect("database url not set")
        .replace("sqlite://", "file://");

    let raw_path = path.clone().replace("file://", "");

    let path1 = Path::new(&raw_path);
    let path_parent = path1.parent().expect("data store folder no parent");
    println!("path parent url = {:?}", path_parent.to_str());
    fs::create_dir_all(path_parent).expect("Directory unable to be created.");
    let remove = fs::remove_file(raw_path);
    println!("file remove result {:?}", remove);
    // TODO: MKdir ~/.rg/sqlx -- that was the issue
    println!("database url = {}", path);
    println!("Is this working?? url = {}", path);

    // TODO: Remove this when migrating to sqlx as sqlx can't seem to create a table ? as otherwise
    let conn = Connection::open(path.clone()).expect("Open");
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS temp_debug (
                  test INTEGER PRIMARY KEY
                  )",
        [],
    )
    .expect("create temp table");

    // let count = conn.execute("INSERT OR REPLACE INTO temp_debug (test) VALUES (?1)", [1]).expect("insert");
    // let count = conn.execute("SELECT COUNT(*) FROM temp_debug", []).expect("count");
    // println!("Count");

    let options = SqliteConnectOptions::new()
        .create_if_missing(true) // TODO: Why does this not work in build.rs
        // it seems to work properly without rusqlite within the main crate ?
        .filename(Path::new(&path.clone()));
    let pool = SqlitePool::connect_with(options)
        .await
        .expect("Connection failure");

    // this works
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Migrations execution failed");
    println!("SQL migrations Build script ran");
}

fn main() {
    println!("Redgold Data datastore SQL migrations build script started");

    Builder::new_current_thread()
        .enable_all()
        .enable_time()
        .build()
        .unwrap()
        .block_on(init_db());
}
