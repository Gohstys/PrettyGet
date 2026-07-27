// PrettyGet — frontend
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => Array.from(document.querySelectorAll(sel));

let upgrades = [];
let lastExplore = [];
let hasSearched = false;
let busy = false;
let exploreMode = "search";
let lang = "en";
let wingetOk = null;
let elevated = false;

// ============ i18n ============
const I18N = {
  en: {
    "nav.updates": "Updates", "nav.explore": "Explore", "nav.schedule": "Schedule", "nav.logs": "Logs",
    "search.global": "Search packages…",
    "admin.run": "⛊ Run as admin", "settings.title": "Settings", "help.title": "Help",
    "status.checking": "Checking winget…", "status.connected": "winget connected", "status.notfound": "winget not found",
    "status.admin": " · admin",
    "updates.title": "Updates", "updates.refresh": "Refresh", "updates.updateAll": "Update all",
    "updates.selectAll": "Select all", "updates.checking": "Checking packages…",
    "updates.count": "{n} package(s) with an available update", "updates.uptodate": "Everything is up to date! 🎉",
    "updates.none": "No pending updates.", "updates.press": "Press Refresh to check for updates.",
    "updates.updateSelected": "Update {n} selected →", "updates.updateSelectedBase": "Update selected →",
    "btn.update": "Update", "btn.install": "Install", "btn.uninstall": "Uninstall",
    "explore.title": "Explore packages", "explore.sub": "Find new apps to install, or review what you already have.",
    "explore.findNew": "Find new", "explore.installed": "Installed", "explore.searchPh": "Type a name, e.g. firefox…",
    "explore.searchPhInstalled": "Filter installed (empty = all)…", "explore.search": "Search",
    "explore.start": "Search a package by name to get started.", "explore.noResults": "No results.",
    "explore.searching": "Searching…", "explore.typeSomething": "Type something to search",
    "adv.title": "Advanced options", "adv.source": "Source", "adv.srcAll": "All sources", "adv.srcStore": "Microsoft Store",
    "adv.mode": "Install mode", "adv.silent": "Silent", "adv.interactive": "Interactive (show installer)",
    "adv.hint": "Use Interactive if a package fails silently, or Microsoft Store for Store-only apps.",
    "sch.title": "Schedule updates", "sch.sub": "Create tasks that update everything automatically.",
    "sch.name": "Name", "sch.freq": "Frequency", "sch.daily": "Every day", "sch.weekly": "Every week", "sch.monthly": "Every month",
    "sch.time": "Time", "sch.create": "Create task", "sch.active": "Active tasks", "sch.none": "No scheduled tasks yet.",
    "sch.test": "▶ Test", "sch.delete": "🗑 Delete", "sch.next": "Next run", "sch.status": "Status", "sch.needName": "Give the task a name",
    "logs.title": "Logs", "logs.sub": "Live output from winget operations.", "logs.clear": "Clear", "logs.abort": "Abort",
    "log.cancelled": "— Cancelled by user —", "toast.cancelled": "Operation cancelled",
    "toast.updateDone": "Success: packages updated", "toast.finishedCode": "Finished with code {code}",
    "toast.selectionDone": "Selection processed", "toast.busy": "An operation is already running",
    "toast.noWinget": "winget not detected. Install it from the Microsoft Store (App Installer).",
    "toast.elevating": "Restarting as administrator…", "toast.alreadyAdmin": "Already running as administrator",
    "help.text": "Refresh to find updates, Update all to apply them, or schedule them in Schedule. Run as admin once to avoid repeated UAC prompts.",
    "settings.about": "PrettyGet v0.1.0 · winget {status}",
    "log.updating": "▶ Updating {x}…", "log.updatingAll": "▶ Updating ALL packages…",
    "log.updatingSel": "▶ Updating {n} selected package(s)…",
    "log.installing": "▶ Installing {x}…", "log.uninstalling": "▶ Uninstalling {x}…",
    "log.finished": "— Process finished (code {code}) —", "log.adminHint": "Tip: run as administrator to avoid a UAC prompt per app.",
    "pro.nav": "Advanced", "pro.title": "Advanced tools", "pro.sub": "Power tools for power users and teams — free for everyone.",
    "donate.nav": "Donate", "donate.title": "Support PrettyGet", "donate.sub": "PrettyGet is free and always will be. If it's useful to you, consider supporting its development.",
    "donate.sponsors.title": "GitHub Sponsors", "donate.sponsors.body": "Recurring or one-time support, right from your GitHub account.", "donate.sponsors.cta": "Sponsor on GitHub",
    "donate.bmc.title": "Buy Me a Coffee", "donate.bmc.body": "A quick way to say thanks with a one-time contribution.", "donate.bmc.cta": "Buy me a coffee",
    "donate.thanks": "Thank you for keeping PrettyGet free and ad-free. ❤️",
    "ss.title": "State Sync", "ss.sub": "Export and import your full winget package set.", "ss.exportJson": "Export JSON", "ss.exportYaml": "Export YAML",
    "ss.import": "Import", "ss.importPh": "Exported JSON/YAML appears here, or paste to import…", "ss.imported": "Import finished (code {code})", "ss.needData": "Nothing to import",
    "rd.title": "Remote Deploy", "rd.sub": "Run winget on remote machines (WinRM).", "rd.hostsPh": "Hosts (comma-separated)…",
    "rd.argsPh": "winget args, e.g. upgrade --all", "rd.run": "Run", "rd.needHosts": "Enter at least one host", "rd.running": "Running on remote hosts…",
    "iac.title": "IaC Generator", "iac.sub": "Turn a selection into PowerShell or Ansible.", "iac.install": "Install", "iac.upgrade": "Upgrade",
    "iac.uninstall": "Uninstall", "iac.generate": "Generate", "iac.pkgsPh": "Package IDs, one per line…", "iac.needPkgs": "Add at least one package ID",
    "dm.title": "Silent Daemon", "dm.sub": "Background Windows service for silent scheduled updates.", "dm.enabled": "Enabled", "dm.apply": "Apply",
    "dm.uninstall": "Uninstall service", "dm.exePh": "Path to prettyget-daemon.exe…", "dm.hint": "Requires administrator. Use “Run as admin” first.",
    "dm.needExe": "Enter the daemon .exe path", "dm.applied": "Daemon configured", "dm.uninstalled": "Service uninstalled",
    "common.copy": "Copy", "common.copied": "Copied",
  },
  es: {
    "nav.updates": "Actualizaciones", "nav.explore": "Explorar", "nav.schedule": "Programar", "nav.logs": "Registro",
    "search.global": "Buscar paquetes…",
    "admin.run": "⛊ Ejecutar como admin", "settings.title": "Ajustes", "help.title": "Ayuda",
    "status.checking": "Comprobando winget…", "status.connected": "winget conectado", "status.notfound": "winget no encontrado",
    "status.admin": " · admin",
    "updates.title": "Actualizaciones", "updates.refresh": "Buscar", "updates.updateAll": "Actualizar todo",
    "updates.selectAll": "Seleccionar todo", "updates.checking": "Comprobando paquetes…",
    "updates.count": "{n} paquete(s) con actualización disponible", "updates.uptodate": "¡Todo está al día! 🎉",
    "updates.none": "No hay actualizaciones pendientes.", "updates.press": "Pulsa Buscar para comprobar actualizaciones.",
    "updates.updateSelected": "Actualizar {n} seleccionadas →", "updates.updateSelectedBase": "Actualizar seleccionadas →",
    "btn.update": "Actualizar", "btn.install": "Instalar", "btn.uninstall": "Desinstalar",
    "explore.title": "Explorar paquetes", "explore.sub": "Busca apps nuevas para instalar o revisa lo que ya tienes.",
    "explore.findNew": "Buscar nuevas", "explore.installed": "Instaladas", "explore.searchPh": "Escribe un nombre, ej. firefox…",
    "explore.searchPhInstalled": "Filtrar instaladas (vacío = todas)…", "explore.search": "Buscar",
    "explore.start": "Busca un paquete por su nombre para empezar.", "explore.noResults": "Sin resultados.",
    "explore.searching": "Buscando…", "explore.typeSomething": "Escribe algo para buscar",
    "adv.title": "Opciones avanzadas", "adv.source": "Fuente", "adv.srcAll": "Todas las fuentes", "adv.srcStore": "Microsoft Store",
    "adv.mode": "Modo de instalación", "adv.silent": "Silenciosa", "adv.interactive": "Interactiva (mostrar instalador)",
    "adv.hint": "Usa Interactiva si un paquete falla en silencio, o Microsoft Store para apps solo de la Store.",
    "sch.title": "Programar actualizaciones", "sch.sub": "Crea tareas que actualicen todo automáticamente.",
    "sch.name": "Nombre", "sch.freq": "Frecuencia", "sch.daily": "Cada día", "sch.weekly": "Cada semana", "sch.monthly": "Cada mes",
    "sch.time": "Hora", "sch.create": "Crear tarea", "sch.active": "Tareas activas", "sch.none": "Aún no hay tareas programadas.",
    "sch.test": "▶ Probar", "sch.delete": "🗑 Eliminar", "sch.next": "Próxima ejecución", "sch.status": "Estado", "sch.needName": "Pon un nombre a la tarea",
    "logs.title": "Registro", "logs.sub": "Salida en vivo de las operaciones de winget.", "logs.clear": "Limpiar", "logs.abort": "Abortar",
    "log.cancelled": "— Cancelado por el usuario —", "toast.cancelled": "Operación cancelada",
    "toast.updateDone": "Listo: paquetes actualizados", "toast.finishedCode": "Finalizó con código {code}",
    "toast.selectionDone": "Selección procesada", "toast.busy": "Ya hay una operación en curso",
    "toast.noWinget": "No se detectó winget. Instálalo desde la Microsoft Store (App Installer).",
    "toast.elevating": "Reiniciando como administrador…", "toast.alreadyAdmin": "Ya se ejecuta como administrador",
    "help.text": "Buscar para ver actualizaciones, Actualizar todo para aplicarlas, o prográmalas en Programar. Ejecuta como admin una vez para evitar el UAC repetido.",
    "settings.about": "PrettyGet v0.1.0 · winget {status}",
    "log.updating": "▶ Actualizando {x}…", "log.updatingAll": "▶ Actualizando TODOS los paquetes…",
    "log.updatingSel": "▶ Actualizando {n} paquete(s) seleccionados…",
    "log.installing": "▶ Instalando {x}…", "log.uninstalling": "▶ Desinstalando {x}…",
    "log.finished": "— Proceso finalizado (código {code}) —", "log.adminHint": "Consejo: ejecuta como administrador para evitar el UAC por cada app.",
    "pro.nav": "Avanzado", "pro.title": "Herramientas avanzadas", "pro.sub": "Herramientas avanzadas para usuarios y equipos — gratis para todos.",
    "donate.nav": "Donar", "donate.title": "Apoya a PrettyGet", "donate.sub": "PrettyGet es gratis y lo seguirá siendo. Si te resulta útil, valora apoyar su desarrollo.",
    "donate.sponsors.title": "GitHub Sponsors", "donate.sponsors.body": "Apoyo recurrente o puntual, directamente desde tu cuenta de GitHub.", "donate.sponsors.cta": "Patrocinar en GitHub",
    "donate.bmc.title": "Buy Me a Coffee", "donate.bmc.body": "Una forma rápida de decir gracias con una aportación puntual.", "donate.bmc.cta": "Invítame a un café",
    "donate.thanks": "Gracias por mantener PrettyGet gratis y sin anuncios. ❤️",
    "ss.title": "Sincronización de estado", "ss.sub": "Exporta e importa todo tu conjunto de paquetes de winget.", "ss.exportJson": "Exportar JSON", "ss.exportYaml": "Exportar YAML",
    "ss.import": "Importar", "ss.importPh": "Aquí aparece el JSON/YAML exportado, o pégalo para importar…", "ss.imported": "Importación finalizada (código {code})", "ss.needData": "Nada que importar",
    "rd.title": "Despliegue remoto", "rd.sub": "Ejecuta winget en máquinas remotas (WinRM).", "rd.hostsPh": "Hosts (separados por comas)…",
    "rd.argsPh": "args de winget, ej. upgrade --all", "rd.run": "Ejecutar", "rd.needHosts": "Introduce al menos un host", "rd.running": "Ejecutando en hosts remotos…",
    "iac.title": "Generador IaC", "iac.sub": "Convierte una selección en PowerShell o Ansible.", "iac.install": "Instalar", "iac.upgrade": "Actualizar",
    "iac.uninstall": "Desinstalar", "iac.generate": "Generar", "iac.pkgsPh": "Ids de paquetes, uno por línea…", "iac.needPkgs": "Añade al menos un Id",
    "dm.title": "Daemon silencioso", "dm.sub": "Servicio en segundo plano para actualizaciones silenciosas programadas.", "dm.enabled": "Activado", "dm.apply": "Aplicar",
    "dm.uninstall": "Desinstalar servicio", "dm.exePh": "Ruta a prettyget-daemon.exe…", "dm.hint": "Requiere administrador. Usa «Ejecutar como admin» primero.",
    "dm.needExe": "Introduce la ruta del .exe del daemon", "dm.applied": "Daemon configurado", "dm.uninstalled": "Servicio desinstalado",
    "common.copy": "Copiar", "common.copied": "Copiado",
  },
};
function t(key, params) {
  let s = (I18N[lang] && I18N[lang][key]) || I18N.en[key] || key;
  if (params) for (const k in params) s = s.replaceAll(`{${k}}`, params[k]);
  return s;
}
function applyI18n() {
  document.documentElement.lang = lang;
  $$("[data-i18n]").forEach((el) => (el.textContent = t(el.dataset.i18n)));
  $$("[data-i18n-ph]").forEach((el) => (el.placeholder = t(el.dataset.i18nPh)));
  $$("[data-i18n-title]").forEach((el) => (el.title = t(el.dataset.i18nTitle)));
  setStatusText();
  setRefreshIdle();
  $("#upgradeAllBtn").innerHTML = `<span class="ico">⤓</span> ${t("updates.updateAll")}`;
  $("#searchInput").placeholder = exploreMode === "search" ? t("explore.searchPh") : t("explore.searchPhInstalled");
  renderUpgrades();
  renderExplore(lastExplore);
}
function setLang(l) {
  lang = l;
  try { localStorage.setItem("pg.lang", l); } catch {}
  $("#lang").value = l;
  applyI18n();
}
$("#lang").addEventListener("change", (e) => setLang(e.target.value));

// ============ Tabs ============
function switchTab(tab) {
  $$(".nav-item").forEach((b) => b.classList.toggle("active", b.dataset.tab === tab));
  $$(".tab").forEach((t) => t.classList.toggle("active", t.id === `tab-${tab}`));
  if (tab === "schedule") loadSchedules();
}
$$(".nav-item").forEach((btn) => btn.addEventListener("click", () => switchTab(btn.dataset.tab)));

// ============ Toast ============
function toast(msg, type = "info") {
  const icon = type === "ok" ? "✓" : type === "err" ? "!" : "i";
  const el = document.createElement("div");
  el.className = `toast ${type}`;
  el.innerHTML = `<span class="t-ico">${icon}</span><span class="t-msg"></span><button class="t-close">✕</button>`;
  el.querySelector(".t-msg").textContent = msg;
  el.querySelector(".t-close").addEventListener("click", () => el.remove());
  $("#toastWrap").appendChild(el);
  setTimeout(() => { el.style.opacity = "0"; el.style.transition = "opacity 0.3s"; setTimeout(() => el.remove(), 300); }, 3800);
}

// ============ Live log ============
let logLines = [];
let logTransient = null;
let cancelling = false;
function renderLog() {
  const box = $("#logBox");
  let txt = logLines.join("\n");
  if (logTransient !== null) txt += (txt ? "\n" : "") + logTransient;
  box.textContent = txt;
  box.scrollTop = box.scrollHeight;
}
function logLine(text) { logLines.push(text); logTransient = null; renderLog(); }
$("#clearLogBtn").addEventListener("click", () => { logLines = []; logTransient = null; renderLog(); });

function setProgress(percent) {
  $("#progressBar").style.width = percent + "%";
  $("#progressLabel").textContent = percent + "%";
}
function beginBusy() {
  busy = true;
  $("#abortBtn").hidden = false;
  $("#progressWrap").hidden = false;
  setProgress(0);
}
function endBusy() {
  busy = false;
  $("#abortBtn").hidden = true;
  $("#progressWrap").hidden = true;
}
$("#abortBtn").addEventListener("click", async () => {
  if (!busy || cancelling) return;
  cancelling = true;
  try {
    const killed = await invoke("cancel_running");
    if (!killed) cancelling = false;
  } catch (err) { cancelling = false; toast(String(err), "err"); }
});

listen("winget-out", (e) => {
  const { text, transient, percent } = e.payload;
  if (transient) {
    logTransient = text;
    if (typeof percent === "number") setProgress(percent);
  } else { logLines.push(text); logTransient = null; }
  renderLog();
});
listen("winget-done", (e) => {
  const code = e.payload;
  logLine("");
  if (cancelling) {
    logLine(t("log.cancelled"));
    toast(t("toast.cancelled"), "info");
    cancelling = false;
  } else {
    logLine(t("log.finished", { code }));
    if (code === 0) toast(t("toast.updateDone"), "ok");
    else toast(t("toast.finishedCode", { code }), "err");
  }
  endBusy();
  refresh();
});

// ============ App icons ============
// Avatar de letra coloreada, generado localmente: no se hace ninguna petición
// externa (antes se consultaba logo.clearbit.com, revelando a un tercero qué
// paquetes tiene instalados o mira el usuario).
function colorFor(name) {
  let h = 0;
  for (const c of String(name)) h = (h * 31 + c.charCodeAt(0)) % 360;
  return `hsl(${h}, 42%, 38%)`;
}
function iconHtml(pkg) {
  const letter = (pkg.name || pkg.id || "?").trim().charAt(0).toUpperCase() || "?";
  // Sin estilo en línea (a propósito: permite una CSP estricta sin 'unsafe-inline').
  // El color de fondo se aplica en JS después de insertar el HTML, ver applyIconColors().
  return `<div class="app-icon" data-seed="${esc(pkg.name || pkg.id)}"><span>${esc(letter)}</span></div>`;
}
function applyIconColors(root) {
  root.querySelectorAll(".app-icon[data-seed]").forEach((el) => {
    el.style.background = colorFor(el.dataset.seed);
  });
}

// ============ Status / admin ============
function setStatusText() {
  const el = $("#wingetStatus");
  if (wingetOk === null) { el.textContent = t("status.checking"); el.className = "winget-status"; return; }
  if (wingetOk) {
    el.textContent = t("status.connected") + (elevated ? t("status.admin") : "");
    el.className = "winget-status ok";
  } else { el.textContent = t("status.notfound"); el.className = "winget-status bad"; }
}
async function checkWinget() {
  wingetOk = await invoke("winget_available");
  if (!wingetOk) toast(t("toast.noWinget"), "err");
  setStatusText();
}
async function checkElevated() {
  try { elevated = await invoke("is_elevated"); } catch { elevated = false; }
  $("#adminBtn").hidden = elevated;
  setStatusText();
}
$("#adminBtn").addEventListener("click", async () => {
  if (elevated) return toast(t("toast.alreadyAdmin"), "info");
  toast(t("toast.elevating"), "info");
  try { await invoke("relaunch_as_admin"); } catch (err) { toast(String(err), "err"); }
});

// ============ Updates ============
function setRefreshIdle() { $("#refreshBtn").innerHTML = `<span class="ico">⟳</span> ${t("updates.refresh")}`; }
async function refresh() {
  const btn = $("#refreshBtn");
  btn.disabled = true;
  btn.innerHTML = `<span class="spin">⟳</span> ${t("updates.checking")}`;
  $("#updatesSubtitle").innerHTML = `<span class="info-dot">ⓘ</span> ${t("updates.checking")}`;
  try { upgrades = await invoke("list_upgrades"); renderUpgrades(); }
  catch (err) { toast(String(err), "err"); logLine("ERROR: " + err); }
  finally { btn.disabled = false; setRefreshIdle(); }
}
$("#refreshBtn").addEventListener("click", refresh);

function renderUpgrades() {
  const list = $("#updatesList");
  const count = upgrades.length;
  const badge = $("#updateCount");
  $("#updatesSubtitle").innerHTML = `<span class="info-dot">ⓘ</span> ${count === 0 ? t("updates.uptodate") : t("updates.count", { n: count })}`;
  if (count > 0) { badge.hidden = false; badge.textContent = count; } else badge.hidden = true;
  $("#upgradeAllBtn").disabled = count === 0;
  $("#selectionBar").hidden = count === 0;
  if (count === 0) { list.innerHTML = `<div class="empty"><div class="empty-ico">${wingetOk === false ? "⚠" : "🎉"}</div><p>${esc(t("updates.none"))}</p></div>`; return; }

  list.innerHTML = upgrades.map((u, i) => `
    <div class="card-pkg">
      <input type="checkbox" class="card-chk rowChk" data-i="${i}" />
      <div class="card-top">${iconHtml(u)}<div class="card-info"><div class="card-name">${esc(u.name) || "(no name)"}</div><div class="card-id">${esc(u.id)}</div></div></div>
      <div class="ver-box"><span class="v-old">${esc(u.current)}</span><span class="v-arrow">→</span><span class="v-new">${esc(u.available)}</span></div>
      <div class="card-bottom"><span class="pill">${esc(u.source) || "winget"}</span><button class="card-action upgradeOne" data-id="${esc(u.id)}">${t("btn.update")}</button></div>
    </div>`).join("");
  applyIconColors(list);
  $$(".upgradeOne").forEach((b) => b.addEventListener("click", () => upgradeOne(b.dataset.id)));
  $$(".rowChk").forEach((c) => c.addEventListener("change", updateSelected));
  $("#selectAll").checked = false;
  updateSelected();
}

$("#selectAll").addEventListener("change", (e) => { $$(".rowChk").forEach((c) => (c.checked = e.target.checked)); updateSelected(); });
function selectedIds() { return $$(".rowChk:checked").map((c) => upgrades[Number(c.dataset.i)].id); }
function updateSelected() {
  const n = selectedIds().length;
  const btn = $("#upgradeSelectedBtn");
  btn.disabled = n === 0 || busy;
  btn.textContent = n > 0 ? t("updates.updateSelected", { n }) : t("updates.updateSelectedBase");
}

async function upgradeOne(id) {
  if (busy) return toast(t("toast.busy"), "info");
  beginBusy(); switchTab("logs");
  logLine(""); logLine(t("log.updating", { x: id }));
  if (!elevated) logLine(t("log.adminHint"));
  try { await invoke("upgrade_package", { id }); }
  catch (err) { endBusy(); toast(String(err), "err"); logLine("ERROR: " + err); }
}
$("#upgradeAllBtn").addEventListener("click", async () => {
  if (busy) return toast(t("toast.busy"), "info");
  beginBusy(); switchTab("logs");
  logLine(""); logLine(t("log.updatingAll"));
  if (!elevated) logLine(t("log.adminHint"));
  try { await invoke("upgrade_all"); }
  catch (err) { endBusy(); toast(String(err), "err"); logLine("ERROR: " + err); }
});
$("#upgradeSelectedBtn").addEventListener("click", async () => {
  const ids = selectedIds();
  if (ids.length === 0 || busy) return;
  beginBusy(); switchTab("logs");
  logLine(""); logLine(t("log.updatingSel", { n: ids.length }));
  if (!elevated) logLine(t("log.adminHint"));
  for (const id of ids) {
    logLine(`— ${id} —`);
    try { await invoke("upgrade_package", { id }); } catch (err) { logLine("ERROR: " + err); }
  }
  endBusy(); toast(t("toast.selectionDone"), "ok"); refresh();
});

// ============ Explore ============
$$(".seg-btn").forEach((b) => b.addEventListener("click", () => {
  $$(".seg-btn").forEach((x) => x.classList.remove("active"));
  b.classList.add("active");
  exploreMode = b.dataset.mode;
  $("#searchInput").placeholder = exploreMode === "search" ? t("explore.searchPh") : t("explore.searchPhInstalled");
  if (exploreMode === "installed") runExplore();
}));
$("#searchBtn").addEventListener("click", runExplore);
$("#searchInput").addEventListener("keydown", (e) => { if (e.key === "Enter") runExplore(); });

async function runExplore() {
  const q = $("#searchInput").value.trim();
  const list = $("#exploreList");
  if (exploreMode === "search" && !q) return toast(t("explore.typeSomething"), "info");
  const btn = $("#searchBtn");
  btn.disabled = true; btn.innerHTML = '<span class="spin">⟳</span>';
  list.innerHTML = `<div class="empty"><div class="empty-ico"><span class="spin">⟳</span></div><p>${esc(t("explore.searching"))}</p></div>`;
  try {
    const args = exploreMode === "search" ? { query: q, source: advSource() } : { query: q };
    const cmd = exploreMode === "search" ? "search_packages" : "list_installed";
    lastExplore = await invoke(cmd, args);
    hasSearched = true;
    renderExplore(lastExplore);
  } catch (err) {
    toast(String(err), "err");
    list.innerHTML = `<div class="empty"><p class="muted">${esc(String(err))}</p></div>`;
  } finally { btn.disabled = false; btn.textContent = t("explore.search"); }
}

function renderExplore(pkgs, keepEmpty) {
  const list = $("#exploreList");
  if (!pkgs.length) {
    if (!keepEmpty) {
      const msg = hasSearched ? t("explore.noResults") : t("explore.start");
      list.innerHTML = `<div class="empty"><div class="empty-ico">⌕</div><p>${esc(msg)}</p></div>`;
    }
    return;
  }
  const installed = exploreMode === "installed";
  list.innerHTML = pkgs.map((p) => `
    <div class="card-pkg">
      <div class="card-top">${iconHtml(p)}<div class="card-info"><div class="card-name">${esc(p.name) || "(no name)"}</div><div class="card-id">${esc(p.id)}</div></div></div>
      <div class="ver-box"><span class="v-single">${esc(p.version) || "—"}</span></div>
      <div class="card-bottom"><span class="pill">${esc(p.source) || "—"}</span>
        ${installed
          ? `<button class="card-action danger pkgUninstall" data-id="${esc(p.id)}" data-name="${esc(p.name)}">${t("btn.uninstall")}</button>`
          : `<button class="card-action pkgInstall" data-id="${esc(p.id)}" data-name="${esc(p.name)}">${t("btn.install")}</button>`}
      </div>
    </div>`).join("");
  applyIconColors(list);
  $$(".pkgInstall").forEach((b) => b.addEventListener("click", () => runPkg("install_package", { id: b.dataset.id, source: advSource(), silent: advSilent() }, b.dataset.name, "log.installing")));
  $$(".pkgUninstall").forEach((b) => b.addEventListener("click", () => runPkg("uninstall_package", { id: b.dataset.id, silent: advSilent() }, b.dataset.name, "log.uninstalling")));
}

async function runPkg(cmd, args, name, verbKey) {
  if (busy) return toast(t("toast.busy"), "info");
  beginBusy(); switchTab("logs");
  logLine(""); logLine(t(verbKey, { x: name || args.id }));
  if (!elevated) logLine(t("log.adminHint"));
  try { await invoke(cmd, args); }
  catch (err) { endBusy(); toast(String(err), "err"); logLine("ERROR: " + err); }
}

// ============ Global top search ============
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

// ============ Advanced options ============
function advSource() { return $("#advSource").value; }
function advSilent() { return $("#advMode").value === "silent"; }
function loadAdvanced() {
  try {
    $("#advanced").open = localStorage.getItem("pg.advOpen") === "1";
    const src = localStorage.getItem("pg.source"); if (src) $("#advSource").value = src;
    const mode = localStorage.getItem("pg.mode"); if (mode) $("#advMode").value = mode;
  } catch {}
  $("#advSource").addEventListener("change", (e) => save("pg.source", e.target.value));
  $("#advMode").addEventListener("change", (e) => save("pg.mode", e.target.value));
  $("#advanced").addEventListener("toggle", (e) => save("pg.advOpen", e.target.open ? "1" : "0"));
}
function save(k, v) { try { localStorage.setItem(k, v); } catch {} }

// ============ Settings / Help ============
$("#settingsBtn").addEventListener("click", async () => {
  await checkWinget();
  toast(t("settings.about", { status: wingetOk ? t("status.connected") : t("status.notfound") }), wingetOk ? "info" : "err");
});
$("#helpBtn").addEventListener("click", () => toast(t("help.text"), "info"));

// ============ Schedule ============
$("#createSchBtn").addEventListener("click", async () => {
  const name = $("#schName").value.trim();
  const frequency = $("#schFreq").value;
  const time = $("#schTime").value;
  if (!name) return toast(t("sch.needName"), "err");
  try { toast(await invoke("create_schedule", { name, frequency, time }), "ok"); loadSchedules(); }
  catch (err) { toast(String(err), "err"); }
});
async function loadSchedules() {
  const list = $("#schedulesList");
  try {
    const tasks = await invoke("list_schedules");
    if (!tasks.length) { list.innerHTML = `<div class="empty"><p class="muted">${esc(t("sch.none"))}</p></div>`; return; }
    list.innerHTML = tasks.map((tk) => `
      <div class="sch-row">
        <div class="row-main"><div class="row-name">${esc(tk.name)}</div>
        <div class="sch-meta">${t("sch.next")}: ${esc(tk.next_run) || "—"} · ${t("sch.status")}: ${esc(tk.status) || "—"}</div></div>
        <button class="icon-btn runNow" data-name="${esc(tk.name)}">${t("sch.test")}</button>
        <button class="icon-btn danger delSch" data-name="${esc(tk.name)}">${t("sch.delete")}</button>
      </div>`).join("");
    $$(".delSch").forEach((b) => b.addEventListener("click", async () => {
      try { toast(await invoke("delete_schedule", { name: b.dataset.name }), "ok"); loadSchedules(); } catch (err) { toast(String(err), "err"); }
    }));
    $$(".runNow").forEach((b) => b.addEventListener("click", async () => {
      try { toast(await invoke("run_schedule_now", { name: b.dataset.name }), "info"); } catch (err) { toast(String(err), "err"); }
    }));
  } catch (err) { toast(String(err), "err"); }
}

function copyFrom(sel) {
  const el = $(sel);
  el.removeAttribute("readonly"); el.select();
  try { document.execCommand("copy"); toast(t("common.copied"), "ok"); } catch {}
  if (el.id === "rdOut" || el.id === "iacOut") el.setAttribute("readonly", "");
  window.getSelection().removeAllRanges();
}

// StateSync
async function ssExport(format) {
  try { $("#ssOut").value = await invoke("export_state", { format }); }
  catch (err) { toast(String(err), "err"); }
}
$("#ssExportJson").addEventListener("click", () => ssExport("json"));
$("#ssExportYaml").addEventListener("click", () => ssExport("yaml"));
$("#ssImport").addEventListener("click", async () => {
  const data = $("#ssOut").value.trim();
  if (!data) return toast(t("ss.needData"), "err");
  try { const code = await invoke("import_state", { data, silent: true }); toast(t("ss.imported", { code }), code === 0 ? "ok" : "err"); }
  catch (err) { toast(String(err), "err"); }
});
$("#ssCopy").addEventListener("click", () => copyFrom("#ssOut"));

// RemoteDeploy
$("#rdRun").addEventListener("click", async () => {
  const hosts = $("#rdHosts").value.split(",").map((s) => s.trim()).filter(Boolean).map((h) => ({ host: h, user: null }));
  if (!hosts.length) return toast(t("rd.needHosts"), "err");
  const wingetArgs = $("#rdArgs").value.trim().split(/\s+/).filter(Boolean);
  $("#rdOut").value = t("rd.running");
  try {
    const results = await invoke("remote_run", { hosts, wingetArgs });
    $("#rdOut").value = results.map((r) => `# ${r.host} (code ${r.code})\n${r.stdout}${r.stderr ? "\n[stderr] " + r.stderr : ""}`).join("\n\n");
  } catch (err) { $("#rdOut").value = String(err); toast(String(err), "err"); }
});

// IaC
$("#iacGen").addEventListener("click", async () => {
  const packages = $("#iacPkgs").value.split("\n").map((s) => s.trim()).filter(Boolean);
  if (!packages.length) return toast(t("iac.needPkgs"), "err");
  const selection = { action: $("#iacAction").value, packages, silent: true };
  const target = $("#iacTarget").value;
  try { $("#iacOut").value = await invoke("generate_iac", { selection, target }); }
  catch (err) { toast(String(err), "err"); }
});
$("#iacCopy").addEventListener("click", () => copyFrom("#iacOut"));

// SilentDaemon
$("#dmApply").addEventListener("click", async () => {
  const daemonExe = $("#dmExe").value.trim();
  if (!daemonExe) return toast(t("dm.needExe"), "err");
  const config = { frequency: $("#dmFreq").value, time: $("#dmTime").value, only: [], enabled: $("#dmEnabled").checked };
  try { await invoke("daemon_apply", { config, daemonExe }); toast(t("dm.applied"), "ok"); }
  catch (err) { toast(String(err), "err"); }
});
$("#dmUninstall").addEventListener("click", async () => {
  try { await invoke("daemon_uninstall"); toast(t("dm.uninstalled"), "info"); }
  catch (err) { toast(String(err), "err"); }
});

// ============ Donate ============
// Los enlaces se abren en el navegador del sistema (plugin opener), no dentro de la ventana de la app.
$$(".donate-card a").forEach((a) => {
  a.addEventListener("click", (e) => {
    e.preventDefault();
    invoke("plugin:opener|open_url", { url: a.href }).catch((err) => toast(String(err), "err"));
  });
});

// ============ Utils ============
function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

// ============ Init ============
try { const saved = localStorage.getItem("pg.lang"); if (saved) lang = saved; } catch {}
loadAdvanced();
applyI18n();
checkWinget();
checkElevated();
refresh();
