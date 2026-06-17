# PrettyGet ⬇

Una interfaz de escritorio **bonita** para `winget`. Actualiza todo tu sistema, mira qué se va a actualizar, sigue el progreso en vivo y programa actualizaciones automáticas. Construida con **Tauri** (Rust + web), tema oscuro minimal.

![tema](https://img.shields.io/badge/tema-oscuro%20minimal-6d7cff) ![stack](https://img.shields.io/badge/stack-Tauri%20%2B%20Rust-9a6dff)

## Funciones

- **Actualizaciones** — lista los paquetes con versión nueva (nombre, Id, versión actual → disponible). Actualiza todo, una selección, o uno a uno.
- **Explorar** — busca paquetes nuevos (`winget search`) e instálalos, o revisa los instalados (`winget list`) y desinstálalos. **Opciones avanzadas**: elige la fuente (todas / winget / Microsoft Store) y el modo de instalación (silenciosa o interactiva); las preferencias se recuerdan.
- **Registro en vivo** — la salida de winget se retransmite línea a línea durante cualquier operación.
- **Programar** — crea tareas en el Programador de tareas de Windows (cada día / semana / mes a una hora) que ejecutan `winget upgrade --all` en silencio. Puedes probarlas o eliminarlas desde la app.
- **Idioma** — inglés por defecto, con selector EN/ES en la barra superior (se recuerda).
- **Ejecutar como administrador** — botón que reinicia la app elevada una sola vez, para evitar el aviso UAC por cada paquete al actualizar.

## Requisitos

1. **Windows 10/11** con **winget** instalado (viene con *App Installer* de la Microsoft Store).
2. **Rust** → https://rustup.rs
3. **Node.js** (solo para la CLI de Tauri) → https://nodejs.org
4. Dependencias del sistema para Tauri en Windows: **Microsoft Edge WebView2** (ya viene en Windows 11) y **Visual Studio Build Tools** con "Desktop development with C++".

## Arrancar en desarrollo

```bash
cd PrettyGet
npm install          # instala la CLI de Tauri
npm run dev          # compila Rust + abre la ventana (tauri dev)
```

La primera compilación de Rust tarda un poco; las siguientes son rápidas.

## Compilar el instalador

```bash
npm run build        # genera un .msi y un .exe (NSIS) en src-tauri/target/release/bundle/
```

## Estructura

```
PrettyGet/
├─ package.json            # scripts dev/build (CLI de Tauri)
├─ src/                    # frontend (web)
│  ├─ index.html
│  ├─ styles.css
│  └─ main.js
└─ src-tauri/              # backend (Rust)
   ├─ Cargo.toml
   ├─ build.rs
   ├─ tauri.conf.json
   ├─ icons/               # iconos de la app
   └─ src/
      ├─ main.rs           # registro de comandos
      ├─ winget.rs         # listar/buscar/instalar/desinstalar/actualizar + parser de tablas
      └─ schedule.rs       # tareas programadas con schtasks
```

## Cómo funciona por dentro

- **Listar**: ejecuta `winget upgrade --include-unknown` y parsea la tabla **independientemente del idioma** del sistema: localiza las columnas por su posición (a partir de la línea de guiones), limpia códigos ANSI, y filtra pies de tabla. Funciona con Windows en inglés, español, etc.
- **Actualizar**: lanza winget con `--silent` (sin `--disable-interactivity`, para que el progreso fluya) y retransmite la salida en vivo: las líneas terminadas en `\r` (progreso) se emiten como transitorias y reemplazan a la anterior; las terminadas en `\n` se confirman. Eventos `winget-out` / `winget-done`.
- **Administrador**: `relaunch_as_admin` reinicia el ejecutable con `Start-Process -Verb RunAs` (un único UAC); `is_elevated` comprueba el estado.
- **Programar**: usa `schtasks /Create` con el prefijo `PrettyGet_` para poder listar y borrar solo sus propias tareas.
- Las ventanas de consola de winget/schtasks se ocultan con la bandera `CREATE_NO_WINDOW`.

## Próximas ideas

- Barra de progreso por paquete e icono en la bandeja del sistema.
- Notificaciones cuando hay actualizaciones nuevas.
- Exportar/importar la lista de paquetes.

## PrettyGet Pro

Capa de pago modular en `src-tauri/src/pro/` (mismo binario; se desbloquea con una licencia firmada). Incluye pestaña **Pro** con activación de licencia y cuatro funciones: State Sync (export/import JSON/YAML), Remote Deploy (winget remoto vía WinRM), IaC Generator (PowerShell/Ansible) y Silent Daemon (servicio Windows). Detalles completos en [docs/PRO_ARCHITECTURE.md](docs/PRO_ARCHITECTURE.md).

Herramientas (crates aparte en `tools/`):

```bash
cd tools/license-signer   && cargo run -- keygen        # genera el par de claves
cd tools/prettyget-daemon && cargo build --release       # binario del servicio
```

Tras `keygen`, pega el array de la clave pública en `pro/license.rs` (`PUBLIC_KEY_BYTES`). Con la clave en ceros, toda licencia falla y la app queda en modo Free (seguro por defecto).

## Notas

- Si una actualización requiere permisos de administrador, winget pedirá elevación (UAC).
- El parser cubre la salida estándar de winget; si Microsoft cambia el formato, ajusta `parse_upgrades` en `winget.rs` (tiene tests unitarios: `cargo test`).
