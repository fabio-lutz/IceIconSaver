# Ice IconSaver

IceIconSaver is a Windows 11+ desktop app that saves and restores the positions of icons on your desktop. It uses the shell folder view (`IFolderView`) rather than list-view messages, so layouts can survive resolution changes, multi-monitor rearrangements, and other desktop refreshes that scramble icon placement.

## Features

- Lists current desktop icons on startup (name, X, Y, and status)
- **Save** — review which icons to include, then write a `.deskicons` layout file
- **Load** — restore only icons whose positions differ from the file, after a review step
- Turns off **Auto arrange icons** when a restore needs free placement
- Status column shows **SAVED** or **UPDATED** after save/load (localized in the UI)
- English and Brazilian Portuguese (`Help` → `Language`)
- Remembers window size, position, and maximized state

## Requirements

- Windows 11 (or recent Windows 10 with a working desktop shell view)
- [Rust](https://www.rust-lang.org/) with the **MSVC** toolchain

This repository pins the toolchain in [`rust-toolchain.toml`](rust-toolchain.toml) to `stable-x86_64-pc-windows-msvc`. The Windows SDK resource compiler (`rc.exe`) is required so the build can embed the application icon. The GNU toolchain is not supported for this project (it fails looking for `dlltool.exe`).

## Build

```bash
cargo build --release
```

The executable is written to:

```text
target/release/IceIconSaver.exe
```

Run from the project directory:

```bash
cargo run --release
```

## Usage

1. Start the app. The main list shows the current desktop icons.
2. **Save** (File → Save, or the Save button):
   - Confirm the icons in the review dialog (you can remove entries you do not want)
   - Choose a `.deskicons` file (default folder: `%USERPROFILE%\.iceiconsaver`)
3. **Load** (File → Load, or the Load button):
   - Pick a `.deskicons` file
   - If every icon already matches the desktop, nothing is applied
   - Otherwise review the icons to move (missing icons are marked) and confirm
4. Switch language under **Help → Language**. The choice is remembered for the next launch.

## Layout files (`.deskicons`)

Layouts are pretty-printed JSON:

```json
{
  "version": 1,
  "icons": [
    {
      "name": "Recycle Bin",
      "id": "...",
      "x": 0,
      "y": 0
    }
  ]
}
```

| Field     | Meaning                                      |
|-----------|----------------------------------------------|
| `version` | File format version (currently `1`)          |
| `name`    | Display name of the desktop item             |
| `id`      | Stable shell item identity used for matching |
| `x`, `y`  | Desktop coordinates                          |

## App data

Stored under `%USERPROFILE%\.iceiconsaver\`:

| File          | Purpose                                      |
|---------------|----------------------------------------------|
| `locale`      | Saved UI language (`en` or `pt-BR`)          |
| `window.json` | Last window placement                        |
| `*.deskicons` | Layout files you save (default directory)    |

If `locale` is missing, the app picks Brazilian Portuguese when the Windows user locale starts with `pt`, otherwise English.

## Project layout

| Path                 | Role                                              |
|----------------------|---------------------------------------------------|
| `src/main.rs`        | Main window, menus, save/load flow                |
| `src/desktop.rs`     | Shell COM: list icons, apply positions            |
| `src/review.rs`      | Review dialog before save/load                    |
| `src/store.rs`       | `.deskicons` read/write and storage directory     |
| `src/window_state.rs`| Persist window placement                          |
| `src/i18n.rs`        | Locale resolve/save and menu text helpers         |
| `locales/`           | English and Portuguese string catalogs            |
| `build.rs`           | Multi-size icon embedding for the `.exe`          |
| `monitor.ico`        | Source icon (window UI and embedded exe icon)     |

## License / author

Ice IconSaver is released under the [GNU General Public License v3](LICENSE).

Ice IconSaver — Fabio Lutz. More information: https://diyers.com.br/pt/2026/10/18/ice-iconsaver/
