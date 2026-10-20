const { invoke } = window.__TAURI__.core;
const $ = (id) => document.getElementById(id);

let toastTimer;
function toast(message, isError = false) {
  const el = $("toast");
  el.textContent = message;
  el.className = isError ? "toast err" : "toast";
  el.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (el.hidden = true), isError ? 6000 : 2500);
}

async function call(command, args, success) {
  try {
    const result = await invoke(command, args);
    if (success) toast(success);
    return result;
  } catch (error) {
    toast(String(error), true);
    throw error;
  }
}

function describeSource(key) {
  if (!key.source) return { text: "Not set", set: false };
  switch (key.source.kind) {
    case "keychain":
      return { text: `Keychain ${key.masked}`, set: true };
    case "environment":
      return { text: `Environment variable ${key.masked}`, set: true };
    case "envFile":
      return { text: `.env file ${key.masked}`, set: true, title: key.source.path };
  }
  return { text: "Set", set: true };
}

function keyRow(key) {
  const li = document.createElement("li");
  const source = describeSource(key);

  const head = document.createElement("div");
  head.className = "key-head";
  const label = document.createElement("div");
  const title = document.createElement("div");
  title.className = "key-label";
  title.textContent = key.label || key.name;
  const name = document.createElement("code");
  name.className = "muted";
  name.textContent = key.name;
  label.append(title, name);
  const status = document.createElement("div");
  status.className = source.set ? "key-source set" : "key-source";
  status.textContent = source.text;
  if (source.title) status.title = source.title;
  head.append(label, status);

  const form = document.createElement("form");
  form.className = "row";
  const input = document.createElement("input");
  input.type = "password";
  input.autocomplete = "off";
  input.placeholder = source.set ? "Replace key" : "Paste key";
  const save = document.createElement("button");
  save.type = "submit";
  save.textContent = "Save";
  form.append(input, save);

  const inKeychain = key.source && key.source.kind === "keychain";
  if (inKeychain || !key.known) {
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "danger";
    remove.textContent = "Remove";
    remove.addEventListener("click", async () => {
      await call("remove_key", { name: key.name }, `${key.name} removed`);
      load();
    });
    form.append(remove);
  }

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!input.value.trim()) return;
    await call("save_key", { name: key.name, value: input.value }, `${key.name} saved`);
    input.value = "";
    load();
  });

  li.append(head, form);
  return li;
}

function renderStatus(state) {
  const pill = $("status");
  const error = $("server-error");
  const server = state.server;
  if (server.state === "running") {
    pill.textContent = `Running on port ${server.port}`;
    pill.className = "pill ok";
    error.hidden = true;
  } else if (server.state === "failed") {
    pill.textContent = "Not running";
    pill.className = "pill err";
    error.textContent = server.error;
    error.hidden = false;
  } else {
    pill.textContent = "Starting…";
    pill.className = "pill";
    error.hidden = true;
  }
}

async function load() {
  const state = await call("get_settings");
  $("version").textContent = `Version ${state.version}`;
  renderStatus(state);
  $("restart-banner").hidden = !state.restartNeeded;

  const keychain = $("keychain-error");
  if (state.keychainError) {
    keychain.textContent = `The system keychain is unavailable (${state.keychainError}). Keys in ${state.envFile} still work.`;
    keychain.hidden = false;
  } else {
    keychain.hidden = true;
  }

  const known = state.keys.filter((k) => k.known);
  const custom = state.keys.filter((k) => !k.known);
  $("known-keys").replaceChildren(...known.map(keyRow));
  if (custom.length) {
    $("custom-keys").replaceChildren(...custom.map(keyRow));
  } else {
    const empty = document.createElement("li");
    empty.className = "empty";
    empty.textContent = "None yet.";
    $("custom-keys").replaceChildren(empty);
  }

  $("autostart").checked = state.autostart;
  const portInput = $("port");
  if (document.activeElement !== portInput) {
    portInput.value = state.portSource === "default" ? "" : state.port;
  }
  const envOverride = state.portSource === "environment";
  portInput.disabled = envOverride;
  $("port-note").textContent = envOverride
    ? `Set by the TM_PORT environment variable (${state.port}).`
    : "Leave empty for the default (6655). Tailrmade expects 6655 unless you change it there too.";

  $("config-dir").textContent = state.configDir;
  $("config-dir").title = state.configDir;
  $("log-dir").textContent = state.logDir;
  $("log-dir").title = state.logDir;
  $("verbose-logging").checked = state.verboseLogging;
  $("frontend").textContent = state.frontend ? `Serving Tailrmade from ${state.frontend}` : "";
  $("frontend").hidden = !state.frontend;

  $("check-update").disabled = !state.updaterEnabled;
  if (!state.updaterEnabled) {
    $("update-status").textContent = "Not enabled in this build.";
  }
}

$("add-custom").addEventListener("submit", async (event) => {
  event.preventDefault();
  const name = $("custom-name").value.trim().toUpperCase();
  await call("save_key", { name, value: $("custom-value").value }, `${name} saved`);
  $("custom-name").value = "";
  $("custom-value").value = "";
  load();
});

$("port-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const raw = $("port").value.trim();
  const port = raw === "" ? null : Number(raw);
  await call("set_port", { port }, "Port saved");
  load();
});

$("autostart").addEventListener("change", async (event) => {
  try {
    await call("set_autostart", { enabled: event.target.checked });
  } finally {
    load();
  }
});

$("verbose-logging").addEventListener("change", async (event) => {
  try {
    await call("set_verbose_logging", { enabled: event.target.checked });
  } finally {
    load();
  }
});

$("restart").addEventListener("click", () => call("restart"));
$("open-config").addEventListener("click", () => call("open_config_dir"));
$("open-logs").addEventListener("click", () => call("open_logs_dir"));
$("open-tm").addEventListener("click", () => call("open_tailrmade"));

let pendingVersion = null;
$("check-update").addEventListener("click", async () => {
  const status = $("update-status");
  const button = $("check-update");
  button.disabled = true;
  if (pendingVersion) {
    status.textContent = "Downloading…";
    try {
      await call("install_update");
    } catch {
      button.disabled = false;
    }
    return;
  }
  status.textContent = "Checking…";
  try {
    pendingVersion = await invoke("check_update");
    status.textContent = pendingVersion
      ? `Version ${pendingVersion} is available.`
      : "You're up to date.";
    if (pendingVersion) button.textContent = `Install ${pendingVersion} and restart`;
  } catch (error) {
    status.textContent = String(error);
  }
  button.disabled = false;
});

window.addEventListener("focus", load);
load();
