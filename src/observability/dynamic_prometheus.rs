use std::collections::{HashMap, HashSet};
use std::fs;
use futures::TryStreamExt;
use log::info;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use redgold_schema::ErrorInfoContext;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment, NodeMetadata};
use serde::{Deserialize, Serialize};
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser, json_from};
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use crate::core::relay::Relay;
use crate::infra::deploy::{SSHLike, SSHProcessInvoke};
use crate::node::Node;
use crate::node_config::NodeConfig;


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
struct PrometheusScrapeConfig {
    ip: String,
    port: u16,
    network: NetworkEnvironment
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
struct PrometheusLabels {
    job: String,
    environment: String
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
struct PrometheusTargetEntry {
    labels: PrometheusLabels,
    targets: Vec<String>
}



async fn update_tick(relay: &Relay) -> Result<(), ErrorInfo> {
    // TODO: smaller interval here, health polling function? or do separately?
    let nmd = relay.ds.peer_store.active_node_metadata(None).await?;
    let folder = relay.node_config.data_folder.all();
    let buf = folder.metrics_list();
    let data = fs::read_to_string(buf.clone()).ok();

    let mut updated = vec![];
    for n in nmd {
        let ti = n.transport_info.clone();
        let scr = PrometheusScrapeConfig {
            ip: ti.expect("t").external_host.clone().expect("h"),
            port: n.port_or(relay.node_config.network) - 1,
            network: relay.node_config.network,
        };
        updated.push(scr);
    }

    if let Some(d) = data {
        let scr = d.json_from::<Vec<PrometheusScrapeConfig>>().ok();
        if let Some(v) = scr {
            for v in v {
                if v.network != relay.node_config.network {
                    updated.push(v);
                }
            }
        }
    }

    let ser = updated.clone().json()?;
    fs::write(buf, ser).error_info("write failure")?;

    let mut updated_targets: HashMap<String, Vec<String>> = HashMap::new();

    for u in &updated {
        let key = u.network.to_std_string();
        let mut targets = updated_targets.get(&key).cloned().unwrap_or(vec![]);
        targets.push(format!("{}:{}", u.ip.clone(), u.port.clone()));
        updated_targets.insert(key, targets.clone());
    }

    let mut ser2 = vec![];
    for (k, v) in updated_targets.iter() {
        let t = PrometheusTargetEntry {
            labels: PrometheusLabels { job: "dynamic".to_string(), environment: k.clone() },
            targets: v.clone(),
        };
        ser2.push(t);
    }

    fs::write(folder.targets(), ser2.json()?).error_info("write failure")?;
    if std::env::var("REDGOLD_GRAFANA_PUBLIC_WRITER").is_ok() {

        let targets_path = folder.targets().to_str().expect("str").to_string();
        info!("Updating grafana public targets {}", targets_path.clone());
        SSHProcessInvoke::new("grafana-public-node.redgold.io", None)
            .scp(targets_path.clone(), targets_path.clone(), true, None).await
            .add("Failed to update grafana public targets at")
            .add(targets_path)
            .log_error().ok();
    };
    Ok(())
}

pub async fn update_prometheus_configs(relay: Relay) -> JoinHandle<Result<(), ErrorInfo>> {
    let interval = IntervalStream::new(
        // TODO: From config
        tokio::time::interval(tokio::time::Duration::from_secs(100))
    );
    tokio::spawn(async move {
        interval.map(|i| Ok(i)).try_fold(relay, |r, _| async move {
            update_tick(&r).await.map(|_| r)
        }).await.map(|_| ())
    })
}

#[ignore]
#[tokio::test]
async fn debug_targets() {
    let nc = NodeConfig::from_test_id(&10u16);
    let nc2 = NodeConfig::from_test_id(&11u16);
    let relay = Relay::new(nc.clone()).await;
    let relay2 = Relay::new(nc2.clone()).await;
    Node::prelim_setup(relay.clone()).await.unwrap();
    // Need to update peer node info to genesis peer node info maybe?
    relay.ds.peer_store.add_peer_new(&relay2.peer_node_info().await.expect(""), &nc.public_key()).await.unwrap();
    update_tick(&relay).await.unwrap();
}
