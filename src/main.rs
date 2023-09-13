mod config;
mod handshake;

use config::{enable_tracing, get_app_config, AppConfig};
use handshake::NodeConfig;
use tokio::{task::JoinSet, time::Instant};
use tracing::{error, info};

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    enable_tracing();
    let app_config: AppConfig = get_app_config();

    let mut set = JoinSet::new();

    for host_config in app_config.hosts {
        let supported_versions = app_config.supported_versions.clone();
        set.spawn(async move {
            let connect_start = Instant::now();
            let node_config = match NodeConfig::init(
                &host_config.host,
                host_config.network_magic,
                &host_config.network_id,
            )
            .await {
                Ok(config) => config,
                Err(_) => return,
            };
            let connect_duration = connect_start.elapsed();
            match handshake::negotiate(node_config, &supported_versions).await {
                Ok((negotiate_duration, total_duration)) => {
                    info!(
                        "Ping {} success! : connect_duration: {}, negotiate_duration: {}, total_duration: {}",
                        &host_config.host,
                        connect_duration.as_millis(),
                        negotiate_duration.as_millis(),
                        total_duration.as_millis()
                    );
                }
                Err(error) => {
                    error!("Ping {} failed! : {:?}", &host_config.host, error);
                }
            }
        });
    }

    while let Some(res) = set.join_next().await {
        match res {
            Ok(_) => info!("Execute"),
            Err(error) => error!("{}", error),
        }
    }
}
