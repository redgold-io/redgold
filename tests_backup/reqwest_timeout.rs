// use reqwest::ClientBuilder;
// use reqwest::Result;
// use std::time::Duration;
//
// #[tokio::test]
// async fn debug_timeouts() -> Result<()> {
//     let user = "ferris-the-crab";
//     let request_url = format!("https://api.github.com/users/{}", user);
//     println!("{}", request_url);
//
//     let timeout = Duration::new(0, 1);
//     let client = ClientBuilder::new().timeout(timeout).build()?;
//     let response = client.head(&request_url).send().await?;
//
//     if response.status().is_success() {
//         println!("{} is a user!", user);
//     } else {
//         println!("{} is not a user!", user);
//     }
//
//     Ok(())
// }
