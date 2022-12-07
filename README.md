# ![valman logo](static/img/favicon/hammer-16.png) valman
![Build status](https://github.com/mhwcat/valman/actions/workflows/build.yml/badge.svg)  
Dashboard for managing dockerized Valheim dedicated server. Works with excellent [mbround18/valheim-docker](https://github.com/mbround18/valheim-docker).
## Building
[Install Rustlang](https://www.rust-lang.org/tools/install).
Build release version:
```bash
cargo build --release
```
## Configuration
Valman needs `config.toml` and `log4s.yml` files placed next to binary. `log4rs.yml` is used to configure logging levels and appenders. Refer to [Log4rs documentation](https://docs.rs/log4rs/latest/log4rs/) for more information.

### `config.toml` properties:
| Property | Description | Default value |
|---|---|---|
| `server_address` | Socket address that internal server binds to | 0.0.0.0:9999 |
| `docker_socket_path` | Path to Docker socker | /var/run/docker.sock |
| `container_name` | Name (not id) of valheim container |  |
| `template_path` | Path to main html template file | templates/main.html |
| `valheim_server_address` | Valheim server address (note that port have to be `gameplay_port + 1`) | 127.0.0.1:2457 |
| `valheim_backups_path` | Path to valheim backups folder |  |
| `valheim_backups_destination_path` | Path to valheim saves folder |  |
| `restart_delay_seconds` | Allowed delay between container restarts (in seconds) | 60 |
| `last_log_lines_count` | Number of logs to show | 100 |
| `username` | Username for web access |  |
| `password` | Password for web access |  |


When using [mbround18/valheim-docker](https://github.com/mbround18/valheim-docker), assuming this bind mount configuration
```
- /home/user/docker-volumes/valheim/saves:/home/steam/.config/unity3d/IronGate/Valheim
- /home/user/docker-volumes/valheim/server:/home/steam/valheim
- /home/user/docker-volumes/valheim/backups:/home/steam/backups
```
backups config should look like this
```
valheim_backups_path = "/home/user/docker-volumes/valheim/backups/"
valheim_backups_destination_path = "/home/user/docker-volumes/valheim/"
```