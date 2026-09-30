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
| `/` | Filter the container list by name/image substring |
| `Enter` | Container details (stub, not yet implemented) |
| `q` / `Ctrl-C` | Quit |

### Key semantics vs k9s

- `d` removes the selected container here (with confirmation), same
  "destructive delete" semantic as k9s's `d`, though k9s's `d` describes a
  resource rather than deleting it — there's no separate describe view in
  c8s (see d7s/c8s bucket 2 for that work).
- There's no `y` binding: k9s's `y` shows the resource's YAML manifest, but
  containers here have no manifest concept to show.
- `/` filters the container list by name/image substring, matching k9s's
  filter key (and d7s's `/`), rather than k9s's fuzzy match across the whole
  resource list.

## Scope

v1 covers containers only — no images, volumes, networks, compose grouping, or stats/CPU graphs.
