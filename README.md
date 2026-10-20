# kurl

A terminal HTTP client written in Rust with keyboard and mouse navigation, inspired by Postman, Bruno, and Posting.

## Features

- TUI built with Ratatui, Tokio, and Reqwest.
- Mouse support: clickable tabs, buttons, modal controls, and draggable split dividers.
- Multi-tab workspace with horizontal scrolling and close actions.
- Hierarchical collections and groups compatible with Posting YAML (`.posting.yaml`).
- Environment variables and templating (`{{VAR}}`, `${VAR}`, `{{$uuid}}`, `{{$timestamp}}`, `{{$randomInt}}`).
- Response viewer with syntax highlighting and live search (`/`, `n`, `N`).
- Command palette for fuzzy-searching saved requests (`Ctrl+P`).
- Universal import and export:
  - Postman Collection v2.1/v2.0 JSON (with folder hierarchy)
  - Postman Environment JSON
  - Bruno requests and collection directories (`.bru`)
  - cURL commands
  - Posting YAML files
- Clean data storage model:
  - Global configuration and history stored in platform-standard directories (`%APPDATA%\kurl` on Windows, `~/.local/share/kurl` on Linux, `~/Library/Application Support/kurl` on macOS).
  - Project directories remain clean; local collections are read from `./collections` when present.

## Installation

### Prerequisites

- Rust 1.80 or newer

### Building from Source

```bash
git clone https://github.com/K-w-e/kurl.git
cd kurl
cargo build --release
```

The compiled binary will be located at `target/release/kurl` (`kurl.exe` on Windows).

### Installing and Updating via Cargo

To install directly into Cargo's binary path:

```bash
cargo install --git https://github.com/K-w-e/kurl.git
```

To update an existing installation to the latest version:

```bash
cargo install --git https://github.com/K-w-e/kurl.git --force
```

To install a specific release tag:

```bash
cargo install --git https://github.com/K-w-e/kurl.git --tag v0.1.0
```

### Running with a Specific Collection

```bash
kurl ./my-collections
kurl /path/to/collections
```

When no path is specified, `kurl` uses `./collections` if it exists in the current working directory, falling back to the user's global collections directory.

## Controls

### Navigation and Actions

| Key | Mouse | Action |
|---|---|---|
| `Tab` / `Shift+Tab` | Click `FOCUS` badge | Cycle focus to next / previous pane |
| `Ctrl+T` | Click `[+]` on tab bar | Open new request tab |
| `Ctrl+W` | Click `[X]` on tab | Close active tab |
| `Ctrl+P` | Click `[Find]` | Open command palette |
| `Ctrl+I` | Click `[Import]` | Open universal import modal |
| — | Click `[Export]` | Open universal export modal |
| `Ctrl+E` | Click `[Env]` | Switch active environment |
| `Ctrl+S` | Click `[Save]` | Save request to collection |
| `Ctrl+C` | Click `[cURL]` | Copy request as cURL command |
| `y` | Click `[Body]` | Copy response body to clipboard |
| `Ctrl+R` / `F5` / `Enter` (URL bar) | Click `[Send]` | Send request |
| `q` | Click `[Quit]` | Quit application |

### Sidebar (Collections and History)

| Key | Mouse | Action |
|---|---|---|
| `h` | Click `[Hist]` / `[Colls]` | Toggle between collections and history |
| `n` / `Ctrl+N` | Click `[+Req]` | Create new request |
| `g` | Click `[+Grp]` | Create new collection group |
| `d` | Click `[-Req]` / `[-Item]` | Delete selected request or history entry |
| `D` | Click `[-Grp]` | Delete selected collection group and files |
| `C` (in history) | Click `[Clear]` | Clear call history |
| `Enter` | Click item row | Load request into active or new tab |

### Response Viewer

| Key | Mouse | Action |
|---|---|---|
| `j` / `k` / `Up` / `Down` | Mouse wheel | Scroll response viewer |
| `/` | Click `[Search]` | Open in-payload search |
| `n` | — | Next search match |
| `N` | — | Previous search match |
| `Esc` | — | Clear search highlight |

### Layout Resizing

- **Sidebar width**: Click and drag the vertical divider (`|`).
- **Request / response split**: Click and drag the horizontal divider (`-`).

## Formats and Compatibility

### Postman

- Imports Postman Collection v2.0 and v2.1 JSON files, mapping items and folders to collection groups.
- Exports requests and collection groups to Postman Collection v2.1.0 format.
- Imports and exports Postman Environment JSON files.

### Bruno

- Imports and exports `.bru` file definitions (`meta`, HTTP verbs, `headers`, `params:query`, `params:path`, `auth`, `body`).
- Imports full Bruno directories, preserving subfolder structure as collection groups.

### cURL and YAML

- Imports and exports raw cURL commands.
- Native storage format uses Posting YAML (`.posting.yaml`).

## Architecture

```
kurl/
├── src/
│   ├── app/           # State, tab sessions, persistence
│   ├── config/        # Configuration and directory resolution
│   ├── events/        # Input handling and action dispatch
│   ├── formats/       # Postman, Bruno, cURL, and format detection
│   ├── http/          # HTTP execution and cURL parsing
│   ├── models/        # Request, response, auth, and environment models
│   └── ui/            # Ratatui rendering (panes, tabs, modals, footer)
├── tests/             # Compatibility and integration test suite
├── Cargo.toml
└── README.md
```

## Testing

Run the test suite:

```bash
cargo test
```

## License

MIT
