use axum::{
    body::{self, Empty, Full, StreamBody},
    extract::{Path, State},
    http::{self, header, HeaderMap, HeaderValue, Request, Response, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Redirect},
};
use chrono::{DateTime, Local};
use humansize::DECIMAL;
use log::{debug, error};
use std::{collections::HashMap, fs, time::SystemTime};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{backup, docker, valve, version_with_commit, SharedState, SimpleDirEntry, STATIC_DIR};

pub(crate) async fn auth<B>(
    state: State<SharedState>,
    req: Request<B>,
    next: Next<B>,
) -> Result<axum::response::Response, (HeaderMap, StatusCode)> {
    let (username, password) = {
        let state = state.read().await;

        (state.config.username.clone(), state.config.password.clone())
    };

    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let mut www_auth_headers: HeaderMap = HeaderMap::new();
    www_auth_headers.append(header::WWW_AUTHENTICATE, HeaderValue::from_static("Basic"));

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        return Err((www_auth_headers, StatusCode::UNAUTHORIZED));
    };

    let credentials = http_auth_basic::Credentials::from_header(auth_header.to_string())
        .map_err(|_| (www_auth_headers.clone(), StatusCode::UNAUTHORIZED))?;

    if credentials.user_id.eq_ignore_ascii_case(&username)
        && credentials.password.eq_ignore_ascii_case(&password)
    {
        Ok(next.run(req).await)
    } else {
        Err((www_auth_headers, StatusCode::UNAUTHORIZED))
    }
}

pub(crate) async fn root_handler(
    State(state): State<SharedState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let render_start = SystemTime::now();

    let (docker, a2s_client, config, last_restart_time, template) = {
        let state = state.read().await;

        (
            state.docker.clone(),
            state.a2s_client.clone(),
            state.config.clone(),
            state.last_restart_time,
            state.template.clone(),
        )
    };

    let container_info = match docker::retrieve_container_info(
        &docker,
        &config.container_name,
        config.valheim_server_last_log_lines_count as usize,
    )
    .await
    {
        Ok(cont_info) => Some(cont_info),
        Err(e) => {
            error!("Failed fetching Docker container info: {}", e);

            None
        }
    };
    let valve_info =
        match valve::retrieve_valve_info(&a2s_client, &config.valheim_server_address).await {
            Ok(valve_info) => Some(valve_info),
            Err(e) => {
                error!("Failed fetching Valve server info: {}", e);

                None
            }
        };

    let mut backup_files: Vec<_> = fs::read_dir(&config.valheim_backups_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(|r| r.unwrap())
        .collect();
    backup_files.sort_unstable_by_key(|dir| {
        dir.metadata()
            .expect("file metadata")
            .created()
            .expect("file metadata created")
    });
    let simple_backup_files: Vec<SimpleDirEntry> = backup_files
        .into_iter()
        .map(|bf| {
            let metadata = bf.metadata().expect("file metadata");
            let creation_time: DateTime<Local> = metadata
                .created()
                .expect("file metadata creation time")
                .into();
            let name = bf.file_name().to_string_lossy().to_string();

            SimpleDirEntry {
                name,
                creation_time: creation_time.naive_local(),
                hr_size: humansize::format_size(metadata.len(), DECIMAL),
            }
        })
        .collect();
    let files_iter = if simple_backup_files.len() > 5 {
        &simple_backup_files[simple_backup_files.len() - 5..]
    } else {
        simple_backup_files.as_slice()
    };
    let mut backup_files_templated = String::new();
    for backup_file in files_iter {
        backup_files_templated.push_str(&format!(
            r#"<tr><td><a href="/backups/{}">{}</a></td><td>{}</td><td>{}</td><td style="text-align: end;"><a href="/backups/restore/{}" class="restore-btn" role="button" style="padding: 10px; width: 100%;">Restore</a></td></tr>"#,
            backup_file.name,
            backup_file.name,
            backup_file.creation_time.format("%Y-%m-%d %H:%M:%S"),
            backup_file.hr_size,
            backup_file.name
        ));
    }

    let restart_btn_html =
        r#"<a id="restart-btn" href="/restart" role="button" style="height: 64px;">Restart</a>"#;
    let restart_btn_wait = format!(
        r#"<small style="line-height: 64px;">Last restart was less than {} seconds ago, please wait...</small>"#,
        config.valheim_server_restart_delay_seconds
    );

    let mut replace_map: HashMap<String, String> = HashMap::new();
    let build_timestamp: DateTime<Local> = DateTime::from(
        DateTime::parse_from_rfc3339(env!("VERGEN_BUILD_TIMESTAMP"))
            .expect("parse build timestamp"),
    );
    replace_map.insert(
        "%version%".to_string(),
        format!(
            "{} (built {})",
            version_with_commit(),
            build_timestamp.format("%Y-%m-%d %H:%M:%S")
        ),
    );
    replace_map.insert(
        "%container_status%".to_string(),
        container_info
            .as_ref()
            .map(|ci| ci.state.clone())
            .unwrap_or_else(|| "n/a".to_string()),
    );
    replace_map.insert(
        "%container_uptime%".to_string(),
        container_info
            .as_ref()
            .map(|ci| ci.uptime.clone())
            .unwrap_or_else(|| "n/a".to_string()),
    );
    replace_map.insert(
        "%server_version%".to_string(),
        valve_info
            .as_ref()
            .map(|vi| vi.version.clone())
            .unwrap_or_else(|| "n/a".to_string()),
    );
    replace_map.insert(
        "%player_count%".to_string(),
        valve_info
            .as_ref()
            .map(|vi| vi.player_count.to_string())
            .unwrap_or_else(|| "n/a".to_string()),
    );
    replace_map.insert(
        "%last_restart_time%".to_string(),
        last_restart_time
            .map(|rt| rt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "n/a".to_string()),
    );
    replace_map.insert(
        "%server_logs%".to_string(),
        container_info
            .as_ref()
            .map(|ci| ci.logs.clone())
            .unwrap_or_else(|| "n/a".to_string()),
    );
    replace_map.insert("%backups%".to_string(), backup_files_templated);
    let restart_btn = match last_restart_time {
        Some(last_restart) => {
            if last_restart
                .signed_duration_since(Local::now().naive_local())
                .num_seconds()
                .abs()
                > config.valheim_server_restart_delay_seconds.into()
            {
                restart_btn_html
            } else {
                &restart_btn_wait
            }
        }
        None => restart_btn_html,
    };
    replace_map.insert("%restart_btn%".to_string(), restart_btn.to_string());
    // TODO: Valheim currently does not report any meaningful player information :(
    // let mut players_str = String::new();
    // for player in valve_info.players {
    //     players_str.push_str(&format!("<td>{}</td>", player.name));
    // }
    // replace_map.insert("%players%".to_string(), players_str);
    if container_info
        .as_ref()
        .map(|ci| ci.state.clone())
        .unwrap_or_else(|| "n/a".to_string())
        .eq_ignore_ascii_case("running")
    {
        replace_map.insert("%container_status_img%".to_string(), "ok".to_string());
    } else {
        replace_map.insert("%container_status_img%".to_string(), "cross".to_string());
    }

    replace_map.insert(
        "%render_time%".to_string(),
        render_start.elapsed().unwrap().as_millis().to_string(),
    );

    Ok(Html(render_template(&template, &replace_map)))
}

pub(crate) async fn restart_handler(
    State(state): State<SharedState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    {
        let state = state.read().await;
        docker::restart_container(&state.docker, &state.config.container_name)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    {
        let mut state = state.write().await;
        state.last_restart_time = Some(Local::now().naive_local());
    }

    Ok(Redirect::to("/"))
}

pub(crate) async fn backups_handler(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut backup_file_path = {
        let state = state.read().await;

        state.config.valheim_backups_path.clone()
    };
    backup_file_path.push(name);

    let mime_type = mime_guess::from_path(&backup_file_path).first_or_text_plain();

    let file = match File::open(&backup_file_path).await {
        Ok(file) => file,
        Err(err) => return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err))),
    };
    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(mime_type.as_ref()).unwrap(),
        )
        .body(body)
        .unwrap())
}

pub(crate) async fn backups_restore_handler(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    {
        let (mut backup_file_path, destination_path) = {
            let state = state.read().await;

            (
                state.config.valheim_backups_path.clone(),
                state.config.valheim_backups_destination_path.clone()
            )
        };
        backup_file_path.push(name);

        backup::restore_backup(&backup_file_path, &destination_path).map_err(|e| {
            error!("Failed restoring backup: {}", e);

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed restoring backup: {}", e),
            )
        })?;
    }

    {
        let mut state = state.write().await;

        docker::restart_container(&state.docker.clone(), &state.config.container_name)
            .await
            .map_err(|e| {
                error!("Failed restarting container: {}", e);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed restarting container: {}", e),
                )
            })?;

        state.last_restart_time = Some(Local::now().naive_local());
    }

    Ok(Redirect::to("/"))
}

pub(crate) async fn static_path(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');
    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    match STATIC_DIR.get_file(path) {
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))
            .unwrap(),
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            )
            .body(body::boxed(Full::from(file.contents())))
            .unwrap(),
    }
}

#[inline]
fn render_template(template_str: &str, replace_map: &HashMap<String, String>) -> String {
    let mut replaced_str = String::from(template_str);
    for (key, val) in replace_map {
        replaced_str = replaced_str.replace(key, val);
    }

    replaced_str
}
