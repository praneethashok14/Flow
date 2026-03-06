# Flow Browser

![License](https://img.shields.io/badge/license-MIT-blue)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)
![Status](https://img.shields.io/badge/status-finished-green)

A web browser built in Rust, designed around a sidebar tab strip.

<!-- screenshot -->

## About

Flow is a web browser written entirely in Rust. It uses [Tauri](https://tauri.app/) and [wry](https://github.com/tauri-apps/wry) for the webview layer, with a custom rendering pipeline being developed alongside it. The interface puts tabs in a vertical sidebar — out of the way, always visible, and easier to manage at scale.

## Features

- Vertical sidebar tab strip
- Written entirely in Rust
- Webview layer powered by Tauri and wry
- Custom rendering pipeline (in development)
- Privacy-first defaults

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust |
| UI shell | Tauri |
| Webview | wry |
| Renderer | Custom |

## Roadmap

- [x] Project scaffolding and basic window
- [x] Sidebar tab strip UI
- [x] Navigation controls (address bar, back/forward)
- [x] wry webview integration
- [x] Custom rendering pipeline (initial)
- [x] Settings and persistence
- [x] First public release

## Getting Started


Building from source will require [Rust](https://rustup.rs/) and the Tauri prerequisites for your platform:

```sh
git clone https://github.com/praneethashok14/flow
cd flow
cargo build
```

## Contributing

Contributions, ideas, and feedback are welcome. Open an issue to start a discussion.

## License

MIT
