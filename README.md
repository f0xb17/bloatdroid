# bloatdroid

A terminal interface for debloating Android devices.

bloatdroid drives [ADB](https://developer.android.com/tools/adb) from a
full-screen terminal UI. It lists the devices that are attached, shows the
installed applications of the selected device, and lets you remove packages or
restrict their background behaviour without typing ADB commands by hand.

Everything runs locally: bloatdroid only talks to the `adb` executable and never
sends data anywhere.

## Features

- **Device selection** — a device list that refreshes in the background, plus a
  device information overlay with the manufacturer, model, Android version, SDK
  level, build fingerprint, and ADB state.
- **Remove Packages** — pick an installed package and uninstall it for the
  current Android user. Every removal asks for confirmation, reads the active
  user from the device first, and is verified by listing the packages again
  afterwards. A preinstalled application loses its installation for that user,
  while its APK stays in the system image.
- **Standby Crates** — four expandable sections that list every application with
  its current state and let you change it:
  - **Standby Buckets** — read and change the Android standby bucket
    (`active`, `working_set`, `frequent`, `rare`, `restricted`). The `restricted`
    bucket is offered on Android 11 (SDK 30) and later. The `exempted` bucket is
    shown but never changed, because only Android assigns it.
  - **AppOps** — read and cycle the `RUN_ANY_IN_BACKGROUND` mode
    (`allow`, `default`, `ignore`).
  - **App Inactive** — toggle an application between `active` and `inactive`,
    which stops it from running in the background until it is opened again.
  - **Doze Whitelist** — move an application between the user Device Idle
    whitelist and no whitelist. Entries managed by the system whitelist are shown
    but never changed.
- **Show System Apps** — one checkbox below the device list decides whether
  system applications are part of every list in the interface.
- **Operations** — a log of what the interface did and what the device answered,
  including errors.

Every change is read back from the device, so a command that was accepted is not
reported as applied until the device confirms it.

## Requirements

- An Android device or emulator with **USB debugging** enabled, reachable through
  `adb devices`.
- The **`adb` executable on your `PATH`**. Android Platform Tools are enough; a
  full Android Studio installation is not required.
- **Rust 1.100 or newer** and Cargo to build from source.

Some sections depend on what the installed Android version supports. Where a
device does not accept a value, the change is reported as failed instead of
being assumed to work.

## Build and run

```sh
cargo build --release
./target/release/bloat
```

`bloat` takes no arguments; it opens the interface on the alternate screen and
restores the terminal when it exits. If no device is connected, the interface
still starts and the device list fills in as soon as one appears.

To run it straight from a checkout:

```sh
cargo run --release
```

## Using the interface

The window is split into four panels: **Device Selection** (top left),
**Remove Packages** (top right), **Operations** (bottom left), and
**Standby Crates** (bottom right). `Tab` and `Shift+Tab` move the focus between
them; the focused panel has a highlighted border.

### Device Selection

| Key       | Action                                          |
| --------- | ----------------------------------------------- |
| `Up`      | Select the previous device                      |
| `Down`    | Select the next device                          |
| `Space`   | Open or close the device information overlay    |

The **Show System Apps** checkbox is the last row of the same panel and is its
own focus target: move the focus to it with `Tab` and press `Space` to toggle it.

### Remove Packages

| Key       | Action                                  |
| --------- | --------------------------------------- |
| `Up`      | Select the previous package             |
| `Down`    | Select the next package                 |
| `Enter`   | Ask to uninstall the selected package   |
| `y` / `n` | Confirm or cancel the pending uninstall |
| `Esc`     | Cancel the pending uninstall            |

### Standby Crates

| Key            | Action                                     |
| -------------- | ------------------------------------------ |
| `Up`           | Move the cursor one line up                |
| `Down`         | Move the cursor one line down              |
| `Space`        | Expand or collapse the focused section     |
| `Left`/`Right` | Change the value of the focused application |

### Operations

| Key    | Action              |
| ------ | ------------------- |
| `Up`   | Scroll the log up   |
| `Down` | Scroll the log down |

### Everywhere

| Key         | Action                                |
| ----------- | ------------------------------------- |
| `Tab`       | Move the focus to the next target     |
| `Shift+Tab` | Move the focus to the previous target |
| `Esc` / `q` | Quit                                  |
| `Ctrl+C`    | Quit                                  |

## Safety notes

- Removals are **per Android user**. They do not modify the system image, and
  they are not a substitute for a backup: uninstalling a package for the active
  user can still remove data that belongs to that application.
- Some packages are required for the device to work normally (launchers, the
  Play Store, system services). Review what you remove; bloatdroid validates the
  request and reports the result, but it does not judge whether a package should
  be removed.
- Do not run bloatdroid against a device you do not own or administer.

## Project documentation

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — how to report bugs and open pull
  requests.
- [`SECURITY.md`](SECURITY.md) — how to report a vulnerability privately.
- [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) — community expectations.
- [`docs/GIT_GUIDELINES.md`](docs/GIT_GUIDELINES.md) — branch, commit, and push
  rules.
- [`docs/DESIGN_GUIDELINES.md`](docs/DESIGN_GUIDELINES.md) — language-agnostic
  design principles.
- [`docs/RUST_GUIDELINES.md`](docs/RUST_GUIDELINES.md) — the Rust style and the
  lints that enforce it.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

The test suite runs on Linux and Windows in CI and does not need a connected
device: ADB is exercised through a mock.

## License

MIT. See [`LICENSE`](LICENSE).
