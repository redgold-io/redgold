use std::collections::{HashMap, HashSet};
use std::fs;
use futures::TryStreamExt;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use redgold_schema::{EasyJson, EasyJsonDeser, ErrorInfoContext, json_from};
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment, NodeMetadata};
use serde::{Serialize, Deserialize};
use crate::core::relay::Relay;


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
        let scr = PrometheusScrapeConfig {
            ip: n.external_address.clone(),
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

    for u in updated {
        let or = &mut vec![];
        let mut targets = updated_targets.get_mut(&u.network.to_std_string()).unwrap_or(or);
        targets.push(format!("{}:{}", u.ip, u.port));
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