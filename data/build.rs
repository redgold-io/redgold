use rusqlite::Connection;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use tokio::runtime::Builder;

async fn init_db() {
    let path = std::env::var("DATABASE_URL")
        .expect("database url not set")
        .replace("sqlite://", "file://");

    let raw_path = path.clone().replace("file://", "");

    let path1 = Path::new(&raw_path).clone();
    let path_parent = path1.parent().expect("data store folder no parent").clone();
    println!("path parent url = {:?}", path_parent.clone().to_str());
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
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("Wtf");
    println!("SQL migrations Build script ran");
}

fn main() {
    println!("Datastore SQL migrations build script started");

    Builder::new_current_thread()
        .enable_all()
        .enable_time()
        .build()
        .unwrap()
        .block_on(init_db());
    //
    // let json = include_str!("schema/json_build_config.json");
    // //prost_build::compile_protos(&["src/structs.proto"], &["src"]).unwrap();
    // // prost_serde::build_with_serde(json);
    // let build_config: BuildConfig = serde_json::from_str(json).unwrap();
    //
    // let mut config = prost_build::Config::new();
    // for opt in build_config.opts.iter() {
    //     match opt.scope.as_ref() {
    //         "bytes" => {
    //             config.bytes(&opt.paths);
    //             continue;
    //         }
    //         "btree_map" => {
    //             config.btree_map(&opt.paths);
    //             continue;
    //         }
    //         _ => (),
    //     };
    //     for path in opt.paths.iter() {
    //         match opt.scope.as_str() {
    //             "type" => config.type_attribute(path, opt.attr.as_str()),
    //             "field" => config.field_attribute(path, opt.attr.as_str()),
    //             v => panic!("Not supported type: {}", v),
    //         };
    //     }
    // }
    //
    // config.extern_path(".serde_as", "serde_with::serde_as");
    //
    // // fs::create_dir_all(&build_config.output).unwrap();
    // // config.out_dir(&build_config.output);
    //
    // config
    //     .compile_protos(&build_config.files, &build_config.includes)
    //     .unwrap_or_else(|e| panic!("Failed to compile proto files. Error: {:?}", e));
    //
    // Command::new("cargo")
    //     .args(&["fmt"])
    //     .status()
    //     .expect("cargo fmt failed");

    //build_config
}
