# c8s

A k9s-style TUI for Docker containers, built in Rust with [Ratatui](https://ratatui.rs), reusing [k9tui](../k9tui)'s chrome.

## Features

- **Container list** — name, image, status, ports, and uptime, polled from the Docker daemon every ~2 seconds.
- **Container actions** — start/stop (`s`), restart (`r`), remove with confirmation (`d`/`Delete`).
- **Live log tail** — full-screen streamed logs for the selected container (`l`, `q`/`Esc` to return).
- **Exec shell** — suspends the TUI and hands the real terminal to `docker exec -it <id> sh` (`e`), resuming the TUI on exit.
- **Connection error screen** — if the Docker daemon is unreachable at startup, shows an error with a retry action instead of panicking.

## Requirements

A running Docker daemon reachable at the default `docker.sock`, and the `docker` CLI on `PATH` for the exec-shell action.

## Usage

```sh
cargo run -p c8s
```

Or, after building:

```sh
cargo build --release -p c8s
./target/release/c8s
```

## Hotkeys

| Key | Action |
| --- | --- |
| `j`/`k`, `↑`/`↓` | Move selection |
| `g`/`G` | Jump to top/bottom |
| `s` | Start (if stopped) or stop (if running) the selected container |
| `r` | Restart the selected container |
| `l` | Tail logs for the selected container |
| `e` | Exec an interactive shell in the selected container |
| `d` / `Delete` | Remove the selected container (confirm) |
| `Enter` | Container details (stub, not yet implemented) |
| `q` / `Ctrl-C` | Quit |

## Scope

v1 covers containers only — no images, volumes, networks, compose grouping, or stats/CPU graphs.
