# Changelog

This fork continues [nicolasferreiraxn/Enshrouded-Save-Editor](https://github.com/nicolasferreiraxn/Enshrouded-Save-Editor).
The history up to 1.0.2 is in the
[original changelog](https://github.com/nicolasferreiraxn/Enshrouded-Save-Editor/blob/main/CHANGELOG.md).

## Unreleased

### No more Node.js

Building and testing the editor now needs only Rust. `package.json`, npm and Vite are gone.

- **The save format moved to Rust** (`src-tauri/src/formato/`): the `.ksd` container, the character block and
  knowledge. The interface sends and receives only the model (level, skills, items, knowledge), and the bytes of the
  save never leave Rust. Before, every block crossed into the WebView and back as a JSON array of numbers.
- **The interface has no build step.** JSX became [`htm`](https://github.com/developit/htm), and React 18.3.1 is
  vendored in `src/vendor/`. The WebView loads `src/` as plain ES modules, so `cargo build --release` is the whole
  build. `data/` moved to `src/data/`.
- **Tests run with `cargo test`.** The format round trip and the data checks are Rust tests. The interface tests run
  in headless Edge, the same Chromium as WebView2. Real saves can be checked by pointing `ENSHROUDED_SAVES` at folders
  with `character*.ksd` files or guardian `A.bin` copies.
- **GitHub Actions** tests and builds every push and pull request. A `vX.Y.Z` tag publishes the release with the exe
  and that version's section of this file.
- The Electron leftovers and the Portuguese README and changelog were removed. The interface itself still ships in
  Portuguese and English.

The editor looks and behaves the same.
