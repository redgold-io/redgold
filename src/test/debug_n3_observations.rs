use redgold_schema::EasyJson;
use crate::core::relay::Relay;

#[ignore]
#[tokio::test]
async fn debug_obs() {
    let r = Relay::dev_default().await;
    let mut seed = r.node_config.seeds_now();
    seed.retain(|s| s.external_address == "n3.redgold.io");
    let seed = seed.get(0).cloned().expect("works");
    let pk = seed.public_key.expect("works");
    // let obs = r.ds.observation.select_latest_observation(pk.clone()).await.expect("works");
    // let obs = obs.expect("works");
    // let h = obs.height().expect("works");
    // let obs = obs.observation().expect("works");
    //
    //
    //
    // println!("{}", h);
    // println!("{}", obs.json_or());
    let obs = r.ds.observation.get_pk_observations(&pk, (1e8) as i64).await.expect("works");
    println!("{}", obs.len());
}