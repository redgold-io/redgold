use sqlx::migrate::Migrator;
use sqlx::SqlitePool;

// static MIGRATOR: Migrator = sqlx::migrate!(); // defaults to "./migrations"
//
// #[tokio::test]
// async fn test_sqlx() {
//     dotenv::dotenv().ok();
//
//     let pool = SqlitePool::connect("sqlite:///Users//.rg/debug/data_store.sqlite")
//         .await
//         .expect("Connection failure");
//     sqlx::migrate!("./migrations")
//         .run(&pool)
//         .await
//         .expect("Wtf");
// }
