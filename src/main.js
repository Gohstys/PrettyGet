// PrettyGet — frontend
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => Array.from(document.querySelectorAll(sel));

let upgrades = [];
let busy = false;
let exploreMode = "search";

// ============ Tab navigation ============
function switchTab(tab) {
  $$(".nav-item").forEach((b) => b.classList.toggle("active", b.dataset.tab === tab));
  $$(".tab").forEach((t) => t.classList.toggle("active", t.id === `tab-${tab}`));
  if (tab === "schedule") loadSchedules();
}
$$(".nav-item").forEach((btn) =>
  btn.addEventListener("click", () => switchTab(btn.dataset.tab))
);

// ============ Toast ============
function toast(msg, type = "info") {
  const icon = type === "ok" ? "✓" : type === "err" ? "!" : "i";
  const el = document.createElement("div");
  el.className = `toast ${type}`;
  el.innerHTML = `<span class="t-ico">${icon}</span><span class="t-msg"></span><button class="t-close">✕</button>`;
  el.querySelector(".t-msg").textContent = msg;
  el.querySelector(".t-close").addEventListener("click", () => el.remove());
  $("#toastWrap").appendChild(el);
  setTimeout(() => {
    el.style.opacity = "0";
    el.style.transition = "opacity 0.3s";
    setTimeout(() => el.remove(), 300);
  }, 3600);
}

// ============ Log ============
function log(line) {
  const box = $("#logBox");
  box.textContent += line + "\n";
  box.scrollTop = box.scrollHeight;
}
$("#clearLogBtn").addEventListener("click", () => ($("#logBox").textContent = ""));

listen("upgrade-log", (e) => log(e.payload));
listen("upgrade-done", (e) => {
  const code = e.payload;
  log(`\n— Process finished (code ${code}) —\n`);
  busy = false;
  if (code === 0) toast("Success: packages updated", "ok");
  else toast(`Finished with code ${code}`, "err");
  refresh();
});

// ============ App icons (hybrid: real logo -> monogram) ============
const DOMAIN_MAP = {
  microsoft: "microsoft.com", google: "google.com", mozilla: "mozilla.org",
  spotify: "spotify.com", apple: "apple.com", adobe: "adobe.com",
  valve: "valvesoftware.com", discord: "discord.com", zoom: "zoom.us",
  notion: "notion.so", slack: "slack.com", vlc: "videolan.org",
  videolan: "videolan.org", "7zip": "7-zip.org", git: "git-scm.com",
  python: "python.org", oracle: "oracle.com", nvidia: "nvidia.com",
  brave: "brave.com", opera: "opera.com", telegram: "telegram.org",
  jetbrains: "jetbrains.com", docker: "docker.com", obsproject: "obsproject.com",
  audacity: "audacityteam.org", blender: "blender.org", gimp: "gimp.org",
  whatsapp: "whatsapp.com", steam: "steampowered.com",
};
function domainFor(id) {
  const publisher = String(id).split(".")[0].toLowerCase();
  return DOMAIN_MAP[publisher] || `${publisher.replace(/[^a-z0-9]/g, "")}.com`;
}
function colorFor(name) {
  let h = 0;
  for (const c of String(name)) h = (h * 31 + c.charCodeAt(0)) % 360;
  return `hsl(${h}, 42%, 38%)`;
}
function iconHtml(pkg) {
  const letter = (pkg.name || pkg.id || "?").trim().charAt(0).toUpperCase() || "?";
  const logo = `https://logo.clearbit.com/${domainFor(pkg.id)}`;
  return `<div class="app-icon" style="background:${colorFor(pkg.name || pkg.id)}">
    <span>${esc(letter)}</span>
    <img src="${logo}" alt="" loading="lazy" referrerpolicy="no-referrer" onerror="this.remove()" />
  </div>`;
}

// ============ winget availability ============
async function checkWinget() {
  const ok = await invoke("winget_available");
  const el = $("#wingetStatus");
  if (ok) { el.textContent = "winget conectado"; el.className = "winget-status ok"; }
  else {
    el.textContent = "winget not found"; el.className = "winget-status bad";
    toast("winget not detected. Install it from the Microsoft Store (App Installer).", "err");
  }
  return ok;
}

// ============ Updates ============
async function refresh() {
  const btn = $("#refreshBtn");
  btn.disabled = true;
  btn.innerHTML = '<span class="spin">⟳</span> Checking…';
  $("#updatesSubtitle").innerHTML = '<span class="info-dot">ⓘ</span> Checking packages…';
  try {
    upgrades = await invoke("list_upgrades");
    renderUpgrades();
  } catch (err) {
    toast(String(err), "err");
    log("ERROR: " + err);
  } finally {
    btn.disabled = false;
    btn.innerHTML = '<span class="ico">⟳</span> Refresh';
  }
}
$("#refreshBtn").addEventListener("click", refresh);

function renderUpgrades() {
  const list = $("#updatesList");
  const count = upgrades.length;
  const badge = $("#updateCount");

  $("#updatesSubtitle").innerHTML = count === 0
    ? '<span class="info-dot">ⓘ</span> Everything is up to date! 🎉'
    : `<span class="info-dot">ⓘ</span> ${count} package${count > 1 ? "s" : ""} with an available update`;

  if (count > 0) { badge.hidden = false; badge.textContent = count; } else badge.hidden = true;
  $("#upgradeAllBtn").disabled = count === 0;
  $("#selectionBar").hidden = count === 0;

  if (count === 0) {
    list.innerHTML = `<div class="empty"><div class="empty-ico">🎉</div><p>No pending updates.</p></div>`;
    return;
  }

  list.innerHTML = upgrades.map((u, i) => `
    <div class="card-pkg">
      <input type="checkbox" class="card-chk rowChk" data-i="${i}" />
      <div class="card-top">
        ${iconHtml(u)}
        <div class="card-info">
          <div class="card-name">${esc(u.name) || "(no name)"}</div>
          <div class="card-id">${esc(u.id)}</div>
        </div>
      </div>
      <div class="ver-box">
        <span class="v-old">${esc(u.current)}</span>
        <span class="v-arrow">→</span>
        <span class="v-new">${esc(u.available)}</span>
      </div>
      <div class="card-bottom">
        <span class="pill">${esc(u.source) || "winget"}</span>
        <button class="card-action upgradeOne" data-id="${esc(u.id)}">Update</button>
      </div>
    </div>`).join("");

  $$(".upgradeOne").forEach((b) => b.addEventListener("click", () => upgradeOne(b.dataset.id)));
  $$(".rowChk").forEach((c) => c.addEventListener("change", updateSelected));
  $("#selectAll").checked = false;
  updateSelected();
}

// ============ Selection ============
$("#selectAll").addEventListener("change", (e) => {
  $$(".rowChk").forEach((c) => (c.checked = e.target.checked));
  updateSelected();
});
function selectedIds() { return $$(".rowChk:checked").map((c) => upgrades[Number(c.dataset.i)].id); }
function updateSelected() {
  const n = selectedIds().length;
  const btn = $("#upgradeSelectedBtn");
  btn.disabled = n === 0 || busy;
  btn.textContent = n > 0 ? `Update ${n} selected →` : "Update selected →";
}

// ============ Update actions ============
async function upgradeOne(id) {
  if (busy) return toast("An operation is already running", "info");
  busy = true; switchTab("logs");
  log(`\n▶ Updating ${id}…\n`);
  try { await invoke("upgrade_package", { id }); }
  catch (err) { busy = false; toast(String(err), "err"); log("ERROR: " + err); }
}
$("#upgradeAllBtn").addEventListener("click", async () => {
  if (busy) return toast("An operation is already running", "info");
  busy = true; switchTab("logs");
  log("\n▶ Updating ALL packages…\n");
  try { await invoke("upgrade_all"); }
  catch (err) { busy = false; toast(String(err), "err"); log("ERROR: " + err); }
});
$("#upgradeSelectedBtn").addEventListener("click", async () => {
  const ids = selectedIds();
  if (ids.length === 0 || busy) return;
  busy = true; switchTab("logs");
  log(`\n▶ Updating ${ids.length} selected package(s)…\n`);
  for (const id of ids) {
    log(`\n— ${id} —`);
    try { await invoke("upgrade_package", { id }); }
    catch (err) { log("ERROR: " + err); }
  }
  busy = false; toast("Selection processed", "ok"); refresh();
});

// ============ Explore (search / installed) ============
$$(".seg-btn").forEach((b) =>
  b.addEventListener("click", () => {
    $$(".seg-btn").forEach((x) => x.classList.remove("active"));
    b.classList.add("active");
    exploreMode = b.dataset.mode;
    $("#searchInput").placeholder = exploreMode === "search"
      ? "Type a name, e.g. firefox…" : "Filter installed (empty = all)…";
    if (exploreMode === "installed") runExplore();
  })
);
$("#searchBtn").addEventListener("click", runExplore);
$("#searchInput").addEventListener("keydown", (e) => { if (e.key === "Enter") runExplore(); });

async function runExplore() {
  const q = $("#searchInput").value.trim();
  const list = $("#exploreList");
  if (exploreMode === "search" && !q) return toast("Type something to search", "info");
  const btn = $("#searchBtn");
  btn.disabled = true; btn.innerHTML = '<span class="spin">⟳</span>';
  list.innerHTML = `<div class="empty"><div class="empty-ico"><span class="spin">⟳</span></div><p>Searching…</p></div>`;
  try {
    const cmd = exploreMode === "search" ? "search_packages" : "list_installed";
    renderExplore(await invoke(cmd, { query: q }));
  } catch (err) {
    toast(String(err), "err");
    list.innerHTML = `<div class="empty"><p class="muted">${esc(String(err))}</p></div>`;
  } finally { btn.disabled = false; btn.textContent = "Search"; }
}

function renderExplore(pkgs) {
  const list = $("#exploreList");
  if (!pkgs.length) {
    list.innerHTML = `<div class="empty"><div class="empty-ico">🔍</div><p>No results.</p></div>`;
    return;
  }
  const installed = exploreMode === "installed";
  list.innerHTML = pkgs.map((p) => `
    <div class="card-pkg">
      <div class="card-top">
        ${iconHtml(p)}
        <div class="card-info">
          <div class="card-name">${esc(p.name) || "(no name)"}</div>
          <div class="card-id">${esc(p.id)}</div>
        </div>
      </div>
      <div class="ver-box"><span class="v-single">${esc(p.version) || "—"}</span></div>
      <div class="card-bottom">
        <span class="pill">${esc(p.source) || "—"}</span>
        ${installed
          ? `<button class="card-action danger pkgUninstall" data-id="${esc(p.id)}" data-name="${esc(p.name)}">Uninstall</button>`
          : `<button class="card-action pkgInstall" data-id="${esc(p.id)}" data-name="${esc(p.name)}">Install</button>`}
      </div>
    </div>`).join("");

  $$(".pkgInstall").forEach((b) => b.addEventListener("click", () => runPkg("install_package", b.dataset.id, b.dataset.name, "Installing")));
  $$(".pkgUninstall").forEach((b) => b.addEventListener("click", () => runPkg("uninstall_package", b.dataset.id, b.dataset.name, "Uninstalling")));
}

async function runPkg(cmd, id, name, verb) {
  if (busy) return toast("An operation is already running", "info");
  busy = true; switchTab("logs");
  log(`\n▶ ${verb} ${name || id}…\n`);
  try { await invoke(cmd, { id }); }
  catch (err) { busy = false; toast(String(err), "err"); log("ERROR: " + err); }
}

// ============ Global top search → Explore ============
$("#globalSearch").addEventListener("keydown", (e) => {
  if (e.key !== "Enter") return;
  const q = e.target.value.trim();
  if (!q) return;
  switchTab("explore");
  $$(".seg-btn").forEach((x) => x.classList.toggle("active", x.dataset.mode === "search"));
  exploreMode = "search";
  $("#searchInput").value = q;
  runExplore();
});

// ============ Settings / Help ============
$("#settingsBtn").addEventListener("click", async () => {
  const ok = await checkWinget();
  toast(`PrettyGet v0.1.0 · winget ${ok ? "connected" : "not found"}`, ok ? "info" : "err");
});
$("#helpBtn").addEventListener("click", () =>
  toast("Refresh to find updates, Update all to apply them, or schedule them in Schedule.", "info")
);

// ============ Schedule ============
$("#createSchBtn").addEventListener("click", async () => {
  const name = $("#schName").value.trim();
  const frequency = $("#schFreq").value;
  const time = $("#schTime").value;
  if (!name) return toast("Give the task a name", "err");
  try { toast(await invoke("create_schedule", { name, frequency, time }), "ok"); loadSchedules(); }
  catch (err) { toast(String(err), "err"); }
});

async function loadSchedules() {
  const list = $("#schedulesList");
  try {
    const tasks = await invoke("list_schedules");
    if (!tasks.length) { list.innerHTML = `<div class="empty"><p class="muted">No scheduled tasks yet.</p></div>`; return; }
    list.innerHTML = tasks.map((t) => `
      <div class="sch-row">
        <div class="row-main">
          <div class="row-name">${esc(t.name)}</div>
          <div class="sch-meta">Next run: ${esc(t.next_run) || "—"} · Status: ${esc(t.status) || "—"}</div>
        </div>
        <button class="icon-btn runNow" data-name="${esc(t.name)}">▶ Test</button>
        <button class="icon-btn danger delSch" data-name="${esc(t.name)}">🗑 Delete</button>
      </div>`).join("");
    $$(".delSch").forEach((b) => b.addEventListener("click", async () => {
      try { toast(await invoke("delete_schedule", { name: b.dataset.name }), "ok"); loadSchedules(); }
      catch (err) { toast(String(err), "err"); }
    }));
    $$(".runNow").forEach((b) => b.addEventListener("click", async () => {
      try { toast(await invoke("run_schedule_now", { name: b.dataset.name }), "info"); }
      catch (err) { toast(String(err), "err"); }
    }));
  } catch (err) { toast(String(err), "err"); }
}

// ============ Utils ============
function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

// ============ Init ============
checkWinget();
refresh();
