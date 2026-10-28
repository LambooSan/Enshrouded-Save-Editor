<p align="center">
  <img src=".github/capa.png" alt="Enshrouded Save Editor" width="100%">
</p>

<h1 align="center">Enshrouded Save Editor</h1>

<p align="center">
  <b>Edit your Enshrouded character with an interface that looks like the game itself.</b><br>
  Backpack, equipment, skill tree, gems and knowledge — on local saves or on your dedicated server.
</p>

<p align="center">
  <a href="../../releases/latest"><img src="https://img.shields.io/github/v/release/LambooSan/Enshrouded-Save-Editor?style=for-the-badge&label=download&color=d4a537" alt="Download"></a>
  <a href="../../actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/LambooSan/Enshrouded-Save-Editor/build.yml?style=for-the-badge&label=build" alt="Build"></a>
  <img src="https://img.shields.io/badge/Enshrouded-1.0-8b3fd1?style=for-the-badge" alt="Enshrouded 1.0">
  <img src="https://img.shields.io/badge/Windows-10%20%7C%2011-2b6cb0?style=for-the-badge&logo=windows" alt="Windows">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-3c9d5d?style=for-the-badge" alt="MIT"></a>
</p>

<p align="center">
  🇺🇸 English · 🇷🇺 <a href="README.ru.md">Русский</a>
</p>

---


**Changes:** see the [changelog](CHANGELOG.md).

## ✨ What it does

| | |
|---|---|
| 🧙 **Character** | level and xp; the **real** skill tree, with the game's own layout and positions — and it refuses to save a tree the game would reject (which would make the game refund every point) |
| 🎒 **Backpack** | the 40 slots, the ALT bar and the action bar in the right place; the worn backpack's container at its real size; a catalog of every item in the game, drag to move, duplicate, remove; filters by category, rarity and tier |
| 🛡️ **Equipment** | the paper doll with all 16 slots (armor, vanity, rings, shield, grappling hook, glider, backpack) — and it won't let you take off a backpack that still has things inside |
| 💎 **Any item in detail** | level, amount, durability, upgrades, rarity and **the gem socketed in a weapon** (swap, remove, change its level) |
| ⚔️ **What the item does** | the attribute lines and the per-upgrade enhancements, read from the game's own resources and shown as the game shows them |
| 📜 **Knowledge** | discoveries and recipes, with search by name or id and a level filter |
| 🏰 **Dedicated server** | with [Enshrouded Server Guardian](https://github.com/nicolasferreiraxn/Enshrouded-Server-Guardian), the server's character copies show up in the same list — edit, kick the player, and they come back with your version |
| 💾 **Always a backup** | a backup before every save, restorable from inside the app |
| 🌍 **Portuguese and English** | switch on the bottom bar; adding a language is one JSON file |

Every new item is created with the extra fields that item type really stores, measured on real saves — that's why
what you create in the editor works in the game.

## 🚀 Getting started

1. Download **`enshrouded-save-editor.exe`** from **[Releases](../../releases/latest)**.
2. Run it. Nothing to install — everything is inside the executable.
3. Pick your save and edit. **Ctrl+S** saves (with a backup first).

> ⚠️ Edit with the **game closed**. The game rewrites the save while it's running.

## 🏰 With or without a server

**Without the guardian**, the editor lists your character files (`character0.ksd`, `character1.ksd`, …). A world file (`world0.ksd`) is not a character and stays out of that list.

It looks in two places:

| | |
|---|---|
| **Steam Cloud** | `<Steam>\userdata\<SteamID>\1203620\remote\` |
| **Local, Enshrouded 1.0** | `%USERPROFILE%\Saved Games\keen games\Enshrouded\` |

The Steam folder comes from the registry (`SteamPath` / `InstallPath`), so an install in `C:\Steam` is found along with the usual `Program Files` locations. The game's own `backup` folder is skipped. Anything else can be opened from **Character → Open file…**.

If the list is empty, the character is on the other tab in the game: local saves and Steam Cloud are separate.

**With the guardian**, the copies stored on the server appear next to your local saves:

1. Edit a player's copy and save.
2. Kick the player (or ask them to relog).
3. On the next join the guardian enforces the stored copy — they come back with your version.

The editor also reads the guardian's `rejected/` folder (`recusados/`) and explains, in plain words, what a rejected
save carried: level and xp difference, extra or missing items, items that only moved between containers, and
diverging discoveries.

<details>
<summary><b>Setting up with the guardian, step by step</b></summary>
<br>

**1. On the server.** Install the guardian: stop the server, put its `dbghelp.dll` next to `enshrouded_server.exe`
and start the server. **Ask every player to join once** — the guardian only has someone's copy after seeing their
save. To confirm, look for the `guardiao-dados/` folder with one subfolder per Steam ID.

**2. SFTP details.** In your host's panel (usually *File Manager → SFTP Details*): address, port, user and password.
The port is **not** 22 on most game hosts — on Nodecraft, for example, it's 2022. If you self-host, it's that
machine's own SSH/SFTP.

**3. In the editor.** **Server** tab → **Configure server**:

| field | what goes in it |
|---|---|
| **Address** | the server address, without `sftp://` |
| **Port (SFTP)** | the one from the panel; 2022 on many game hosts |
| **User** | the SFTP user (often not your panel login) |
| **Password** | the SFTP password — stored only on your machine |
| **Server key** | leave empty the first time |
| **Guardian folder** | `guardiao-dados`, unless you changed it |

**Save → Test connection.** It reports the guardian's mode: `aplicar` (enforcing the copy) or `observar` (only
logging). Then **Pin seen key → Save**, so a different SSH key in the future drops the connection instead of being
silently accepted.

**4. Editing a player.** Pick the player, edit, **Ctrl+S**, and kick the player or ask them to relog.

**Before you rely on it:**
- A connected player overwrites the copy every ~30 s. The editor warns you when the copy changes and won't discard
  your edit — but the reliable moment to edit is with the player offline.
- Backups of every save go to `Documents\Enshrouded-Backups\servidor\`.
- The password is stored in plain text in `%APPDATA%\enshrouded-save-editor\servidor.json`, like most desktop
  programs. If that bothers you, use an SFTP account restricted to that server.
</details>

## 🌍 Translating

This file is the main README. The same text is also in Russian ([`README.ru.md`](README.ru.md)).

The interface ships in Portuguese and English. Adding a language is one file, and the translation key **is the
Portuguese text itself** — so anything left untranslated shows in Portuguese, and nothing breaks.

1. Copy `src/i18n/en.json` to `src/i18n/<code>.json` (`es.json`, `de.json`…).
2. Translate the **values**, leave the keys alone, and keep the `{placeholders}` (they can move within the sentence).
3. Register it in `IDIOMAS`, at the top of [`src/i18n.js`](src/i18n.js):

```js
import es from './i18n/es.json' with { type: 'json' };

export const IDIOMAS = [
  { codigo: 'pt-BR', nome: 'Português', locale: 'pt-BR', dicionario: null },
  { codigo: 'en', nome: 'English', locale: 'en-US', dicionario: en },
  { codigo: 'es', nome: 'Español', locale: 'es-ES', dicionario: es },
];
```

Item and skill names come from the game's own localization (`src/data/items.json`, in Portuguese and English).

## 🛠️ Building

Releases are built by **GitHub Actions** from this repository, with a
[public log](../../actions/workflows/build.yml).

Locally, with Rust (`x86_64-pc-windows-msvc`) — no Node, no npm:

```
cd src-tauri
cargo build --release   # target/release/enshrouded-save-editor.exe
```

There is no frontend build step: the interface in `src/` is plain ES modules (React via `htm`, vendored in
`src/vendor/`) that the WebView loads as they are, and the save format lives in Rust (`src-tauri/src/formato/`). No
asset pipeline, no Python: the interface assets come ready in `build-assets/`. For development, `cargo run` from
`src-tauri/` (it rebuilds after any change in `src/`).

To test, `cargo test` from `src-tauri/`. It includes a round trip that requires the rebuilt character block to be
**byte-for-byte identical** to the original; on its own it runs against a synthetic block covering every combination
of an item's extra fields, which checks the reader against the writer. Proving the format itself takes real saves:
set `ENSHROUDED_SAVES` to folders with `character*.ksd` or guardian `A.bin` copies. The interface tests run in
headless Edge — the same Chromium as WebView2 — from `tests/navegador/`.

<details>
<summary><b>Save format notes</b></summary>
<br>

The format isn't documented anywhere; everything here came from measurement.

- **The equipment list is dense and indexed by a bitmask.** Piece `k` takes the k-th set bit of a 32-bit mask,
  followed by a u16 of free slots.
- **Flag `0x08` is vanity (4 bytes).** Without knowing it, the parser "found" equipment in the middle of the backpack
  and wrote a broken save.
- **An item's level already includes its upgrades.** Changing upgrades without changing the level makes the game
  discard the edit.
- **A socketed gem is 5 bytes:** the gem's itemId (u32) + its level (u8).
- **The worn backpack sets the size of the extra container**, found by item type, not by index.
- **Knowledge** is an 8-byte header (`containerState`, `n`), then `n` ids and `n` values. Recipes aren't stored as
  "unlocked": the game recomputes them from discoveries.
</details>

<details>
<summary><b>Game data</b></summary>
<br>

| folder | what it is |
|---|---|
| `src/data/*.json` | item table, skill tree, knowledge map, extra fields per item type, and each item's attribute lines and enhancements (~2.5 MB) |
| `build-assets/game/` | icons, interface textures and fonts, trimmed to what's used (~26 MB, embedded in the exe) |

Prepared from an installed copy of the game. Regenerating isn't part of this repository: after an update that changes
ids or names, the data goes stale until someone regenerates it.
</details>

## 🧩 Same family

| | |
|---|---|
| 🗺️ **[Enshrouded Interactive Map](https://github.com/nicolasferreiraxn/Enshrouded-Interactive-Map)** | real-time map on your second monitor: your position, friends and everything already done |
| 🛡️ **[Enshrouded Server Guardian](https://github.com/nicolasferreiraxn/Enshrouded-Server-Guardian)** | makes the dedicated server the authority over each player's character |

## ❤️ Credits

- **[Nicolas Ferreira](https://github.com/nicolasferreiraxn)** — author of the
  [original editor](https://github.com/nicolasferreiraxn/Enshrouded-Save-Editor) this fork continues, and of the
  save-format research it is built on.
- **[kfc-parser / Enshrouded Mod Loader](https://github.com/Brabb3l/kfc-parser)** by brabb3l — used to read the
  game's resources.
- **Keen Games** — for Enshrouded.

---

<p align="center">
  <sub>
    Fan project, not affiliated with Keen Games. No official support — use, read, modify and publish freely.<br>
    Code under <a href="LICENSE">MIT</a>.
  </sub>
</p>
