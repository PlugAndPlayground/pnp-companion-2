# tm-companion-2

This is the local companion for Tailrmade. It handles API forwarding, local AI
provider requests, and optionally serves a Tailrmade frontend.

The local AI assistant supports these `.env` keys:

```text
ANTHROPIC_API_KEY=
DEEPSEEK_KEY=
GEMINI_API_KEY=
```

## TM Local

Build Tailrmade in self-hosted mode, then copy the contents of its `dist`
directory into `tm` beside this repository's companion files:

```text
tm-companion-2/
  tm/
    index.html
    main.js
    assets/
    ...
```

Then run:

```powershell
.\scripts\build-tm-local.ps1
```

or:

```bash
./scripts/build-tm-local.sh
```

The script emits:

```text
artifacts/
  companion-only/
    tm-companion[.exe]
  tm-local/
    tm-companion[.exe]
    tm/
      index.html
      main.js
      ...
```

The same companion binary is used in both outputs. To update TM later, replace
the contents of the `tm` directory beside the executable and restart companion.

For development, `TM_DIST_DIR` can point directly to any TM `dist` directory.
The application uses port `6655`; use `TM_PORT` to override it and
`TM_NO_OPEN=1` to prevent opening the browser automatically.

To clone the private TM source over SSH, build it, and package both companion
distributions in one step, run:

```powershell
.\scripts\build-tm-from-source.ps1
```

or:

```bash
./scripts/build-tm-from-source.sh
```

The `main` branch is built by default. Use PowerShell `-Ref` or Bash
`TM_REF` to select another branch or tag. The scripts also accept `-Repository`
/ `TM_REPOSITORY` and `-OutputRoot` / `OUTPUT_ROOT`.
