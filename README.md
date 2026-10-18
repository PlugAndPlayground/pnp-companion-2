# Tailrmade Companion

The local companion for [Tailrmade](https://tailrmade.app). It forwards API
calls, runs AI requests with your own provider keys, and can optionally serve a
Tailrmade build locally ("Tailrmade Local").

It runs in the system tray (menu bar on macOS). The tray menu shows the server
status and opens Settings, where you add API keys.

## Install

Download the installer for your OS from
[Releases](https://github.com/PlugAndPlayground/pnp-companion-2/releases):

| OS      | File                                   |
| ------- | -------------------------------------- |
| macOS   | `.dmg` (universal, Apple silicon + Intel) |
| Windows | `.msi` or `-setup.exe`                 |
| Linux   | `.AppImage`, `.deb` or `.rpm`          |

Until the builds are signed, the OS will warn on first launch:

- **macOS:** right-click the app → Open, or run
  `xattr -dr com.apple.quarantine "/Applications/Tailrmade Companion.app"`.
- **Windows:** SmartScreen → More info → Run anyway.

## API keys

Open Settings from the tray and paste keys there. They are stored in the system
keychain (macOS Keychain, Windows Credential Manager, Secret Service on Linux)
and only sent to the matching provider.

Supported keys: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`,
`DEEPSEEK_API_KEY`, `MOONSHOT_API_KEY`. Custom keys added in Settings are
available to HTTP nodes as `$TM_KEY{NAME}`.

Lookup order for a key:

1. Process environment (provider keys above only, handy for development)
2. System keychain
3. `.env` in the config directory
4. Legacy `.env` next to the executable or in the working directory

`$TM_KEY{NAME}` never reads arbitrary environment variables such as `HOME`.

## Config directory

| OS      | Path                                         |
| ------- | -------------------------------------------- |
| macOS   | `~/Library/Application Support/Tailrmade`    |
| Windows | `%APPDATA%\Tailrmade`                         |
| Linux   | `~/.config/tailrmade`                         |

It holds `config.json` (port, custom key names) and a `.env` template for
setups without a keychain. Override it with `TM_CONFIG_DIR`.

## Options

| Setting                  | Where                                         |
| ------------------------ | --------------------------------------------- |
| Port (default 6655)      | Settings, or `TM_PORT` (wins over Settings)   |
| Start at login           | Settings                                      |
| Serve a TM build         | `TM_DIST_DIR`, a `tm/` folder next to the executable, or bundled (Tailrmade Local) |
| Don't open the browser   | `TM_NO_OPEN=1`                                |
| Server only, no tray     | `tm-companion --headless` or `TM_HEADLESS=1`  |

Headless mode suits Linux servers and containers. On Windows release builds the
binary has no console, so logs are not visible in headless mode there.

## Security

Browser origins are restricted to the exact hosts `tailrmade.app`,
`plugandplayground.dev`, `localhost`, and `127.0.0.1` over HTTP or HTTPS, on any
port. Subdomains and other IP addresses are not allowed. Requests with a
disallowed or invalid `Origin` receive HTTP 403. Requests without `Origin`
remain supported for local navigation and non-browser clients; this policy is
not authentication. The server only listens on `127.0.0.1`.

## Development

Requirements: Rust (stable), Node 24, and on Linux:

```bash
sudo apt-get install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev libxdo-dev
```

```bash
npm ci
cargo run                       # tray app
cargo run -- --headless         # server only
cargo test
npx tauri build                 # installers for the current OS
```

The settings window is plain HTML/JS in `ui/`; there is no frontend build step.

## Tailrmade Local

Build Tailrmade in self-hosted mode, then build installers with it bundled:

```bash
./scripts/build-tm-local-installer.sh path/to/tailrmade/dist
```

```powershell
.\scripts\build-tm-local-installer.ps1 -TmDir path\to\tailrmade\dist
```

This produces "Tailrmade Local" installers that open Tailrmade in the browser
on launch. `scripts/build-tm-from-source.*` clone and build Tailrmade first.
The older `scripts/build-tm-local.*` still produce a portable folder (binary +
`tm/`).

## Releases and CI

`.github/workflows/build.yml` builds installers for every push and PR (download
them from the run's artifacts). Pushing a tag `vX.Y.Z` also creates a draft
GitHub Release; bump `version` in `Cargo.toml` first.

Signing switches on per platform once the secrets exist; the workflow header
lists them.

### Updater

1. Generate a key pair once: `npx tauri signer generate -w ~/.tauri/tm-companion.key`
2. Add the private key and its password as secrets `TAURI_SIGNING_PRIVATE_KEY`
   and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
3. Add the public key as the repository variable `TM_UPDATER_PUBKEY`.

Release builds then include `latest.json`, and Settings → Updates installs new
versions from the latest GitHub Release. Builds without the key have the
updater switched off. Tailrmade Local builds made with the script also have it
off.

### macOS signing

Needs an Apple Developer account. Export the "Developer ID Application"
certificate as `.p12`, base64 it (`base64 -i cert.p12 | pbcopy`) and add the
`APPLE_*` secrets listed in the workflow.

### Windows signing

Not wired yet. Options: Azure Trusted Signing (set `bundle.windows.signCommand`
to the `trusted-signing-cli` call) or an OV/EV certificate (`certificateThumbprint`).
