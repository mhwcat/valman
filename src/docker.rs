use std::time::Duration;

use crate::error::{
    Result,
    ValmanError::{Docker as DockerError, DockerApi},
};
use docker_api::{opts::LogsOpts, Docker};
use futures_util::StreamExt;
use log::{debug, error};

#[derive(Debug)]
pub struct ContainerInfo {
    pub id: String,
    pub state: String,
    pub uptime: String,
    pub logs: String,
}

impl ContainerInfo {
    pub fn new(id: String, state: String, uptime: String, logs: String) -> Self {
        Self {
            id,
            state,
            uptime,
            logs,
        }
    }
}

async fn find_container_id_and_state_by_name(
    docker: &Docker,
    name: &str,
) -> Result<(String, String, String)> {
    let container = match docker.containers().list(&Default::default()).await {
        Ok(containers) => {
            let cont = containers
                .into_iter()
                .find(|c| {
                    c.names
                        .as_ref()
                        .unwrap()
                        .first()
                        .unwrap()
                        .eq_ignore_ascii_case(&format!("/{}", name))
                })
                .ok_or_else(|| DockerError("Missing Docker container".to_string()))?;

            Ok(cont)
        }
        Err(e) => Err(DockerApi(e)),
    }?;

    let id = container
        .id
        .ok_or_else(|| DockerError("Missing Docker container ID".to_string()))?;
    let state = container
        .state
        .ok_or_else(|| DockerError("Missing Docker container state".to_string()))?;
    let uptime = container
        .status
        .ok_or_else(|| DockerError("Missing Docker container status".to_string()))?;

    Ok((id, state, uptime))
}

pub async fn retrieve_container_info(
    docker: &Docker,
    container_name: &str,
    last_n_lines: usize,
) -> Result<ContainerInfo> {
    let (container_id, container_state, container_uptime) =
        find_container_id_and_state_by_name(docker, container_name).await?;
    let container = docker.containers().get(&container_id);

    debug!(
        "Retrieving container info for name {} and id {}",
        container_name, container_id
    );

    let cont_logs = container.logs(
        &LogsOpts::builder()
            .stdout(true)
            .n_lines(last_n_lines)
            .build(),
    );
    let logs: Vec<_> = cont_logs
        .map(|chunk| match chunk {
            Ok(chunk) => chunk.to_vec(),
            Err(e) => {
                error!("Error: {}", e);
                vec![]
            }
        })
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let logs = String::from_utf8_lossy(&logs);

    Ok(ContainerInfo::new(
        container_id,
        container_state,
        container_uptime,
        logs.to_string(),
    ))
}

pub async fn restart_container(docker: &Docker, container_name: &str) -> Result<()> {
    let (container_id, _, _) = find_container_id_and_state_by_name(docker, container_name).await?;
    let container = docker.containers().get(&container_id);

    debug!(
        "Restarting container {} with id {}",
        container_name, container_id
    );

    container.restart(Some(Duration::from_secs(10))).await?;

    Ok(())
}
