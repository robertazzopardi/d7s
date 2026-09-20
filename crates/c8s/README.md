# c8s

A k9s-style TUI for Docker containers, built in Rust with [Ratatui](https://ratatui.rs), reusing [k9tui](../k9tui)'s chrome.

![c8s — container list, restart, live logs](demo.gif)

```sh
just c8s-demo
```

Requires [vhs](https://github.com/charmbracelet/vhs) and Docker. Starts fake `demo-web`/`demo-cache` containers, records, tears them down (`just c8s-demo-prep` / `just c8s-demo-teardown` to run those steps separately).

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

## Configuration

### Skin (colors)

Drop a `~/.config/c8s/skin.yml` to override theme colors. Any color omitted
keeps its built-in default; if the file is absent, c8s uses the built-in
theme unchanged. Colors are ANSI names (`white`, `darkgray`, `cyan`, ...) or
`#rrggbb` hex.

```yaml
colors:
  text: white
  focus: "#ffcc00"
  error: red
  success: green
```

Available color names: `muted`, `text`, `focus`, `on_focus`, `selection_fg`,
`selection_bg`, `bg_alt`, `draft`, `multi_select`, `info`, `error`,
`success`, `warning`, `link`.

### Key bindings

Drop a `~/.config/c8s/keys.yml` to remap the action hotkeys (not navigation,
quit, or the `Delete` fallback). It's a flat map of action name to a single
character or a named key (`esc`, `f1`–`f12`, ...):

```yaml
start_stop: s
remove: x
```

Remappable actions: `start_stop`, `restart`, `logs`, `exec`, `remove`.

## Scope

v1 covers containers only — no images, volumes, networks, compose grouping, or stats/CPU graphs.
