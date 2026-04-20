<p align="center">
  <strong>SPARK</strong> — plataforma de operaciones para desarrolladores
</p>

<p align="center">
  <a href="https://dpeluche.github.io/spark/"><strong>dpeluche.github.io/spark</strong></a> ·
  <a href="https://github.com/dPeluChe/spark">GitHub</a> ·
  <a href="https://www.npmjs.com/package/@dpeluche/spark">npm</a> ·
  <a href="README.md">English</a>
</p>

<p align="center">
  <a href="https://github.com/dPeluChe/spark/actions"><img src="https://github.com/dPeluChe/spark/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/dPeluChe/spark/releases"><img src="https://img.shields.io/github/v/release/dPeluChe/spark" alt="Release"></a>
  <a href="https://www.npmjs.com/package/@dpeluche/spark"><img src="https://img.shields.io/npm/v/@dpeluche/spark" alt="npm"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="Licencia"></a>
</p>

<p align="center">
  <a href="#por-qué">Por qué</a> ·
  <a href="#instalación">Instalación</a> ·
  <a href="#módulos">Módulos</a> ·
  <a href="#cli">CLI</a> ·
  <a href="#controles">Controles</a> ·
  <a href="CONTRIBUTING.md">Contribuir</a>
</p>

---

## Por qué

SPARK nació como una herramienta personal. Gestionar decenas de repositorios git, ver cómo los cachés de build se acumulan, olvidar qué servidores de desarrollo siguen corriendo, revisar fechas de expiración de certificados SSL en múltiples proyectos — todo estaba disperso entre distintas herramientas, scripts y pestañas del navegador.

Queríamos una sola interfaz de terminal capaz de manejar el ciclo completo de operaciones de desarrollo: escanear salud de repositorios, limpiar artefactos, hacer seguimiento del estado remoto de todos los repos, monitorear procesos activos, auditar seguridad y mantener las herramientas de desarrollo actualizadas. Así que lo construimos en Rust.

La página oficial tiene más detalles: <https://dpeluche.github.io/spark/>

## Cómo se ve

```
┌─ SPARK v0.5.1 ────────────────────────────────────────────────────────────┐
│  Scanner   Repos   Ports   System   Audit   Updater                       │
├────────────────────────────────────────────────────────────────────────────┤
│  REPOS (47)                                              Total: 2.3 GB    │
│                                                                            │
│  ▸ github.com/miorg/                                                       │
│    > api-service          A  98  main   2d ago    12.4 MB                  │
│    > frontend             B  81  feat   4h ago   890.2 MB  node_modules    │
│    > backend              C  62  main   3w ago     1.1 GB  target/ .venv   │
│    > proyecto-viejo       F  18  main   8mo ago  340.0 MB  obsoleto        │
│                                                                            │
│  [ENTER] Detalle  [a] Agregar ruta  [c] Limpiar  [x] Borrar  [TAB] Siguiente│
└────────────────────────────────────────────────────────────────────────────┘
```

## Instalación

| Método | Comando |
|--------|---------|
| **npm** | `npm install -g @dpeluche/spark` |
| **curl** | `curl -fsSL https://raw.githubusercontent.com/dPeluChe/spark/main/scripts/install.sh \| sh` |
| **cargo** | `cargo install --git https://github.com/dPeluChe/spark` |
| **Binario** | [GitHub Releases](https://github.com/dPeluChe/spark/releases) — macOS arm64/x64, Linux x64 |

Después de instalar:

```bash
spark init    # integración de shell, completions, config
spark         # abrir el TUI
spark doctor  # validar la instalación
```

## Módulos

### Scanner — Analizador de Salud de Repositorios

Descubre repos git en tu sistema, califica su salud (A–F, 0–100) y limpia artefactos de build obsoletos.

- **Calificaciones**: antigüedad del último commit, estado del branch, tamaño de artefactos
- **Limpieza de artefactos**: `node_modules`, `.venv`, `target/`, `.next`, `dist`, cachés de build (20+ tipos)
- **Detección de workspaces**: npm, pnpm, turborepo, nx, cargo, go
- **Rutas personalizadas**: agrega cualquier directorio con `[a]`

### Gestor de Repos — Organizador estilo ghq

Clona, rastrea y gestiona todos tus repositorios desde un solo lugar, organizados por `host/propietario/nombre`.

```
~/repos/
├── github.com/
│   ├── miorg/api-service    main  2d ago
│   ├── miorg/frontend       feat  4h ago
│   └── oss/ripgrep          main  al día
└── gitlab.com/
    └── empresa/interno      dev   behind ↓3
```

- Estado en tiempo real: adelante/atrás/sucio, caché de 4 horas
- **Etiquetas**: agrupa repos por proyecto, cliente o tema
- `spark pull all --tag trabajo` — sincroniza todos los repos de un grupo
- `spark status --tag herramientas-ia` — revisa el estado de un grupo

### Escáner de Puertos — Monitor de Servidores Dev

Encuentra y gestiona servidores de desarrollo y procesos corriendo en tu máquina.

```bash
spark ps

  SERVIDORES DEV (3)
  PUERTO  PID      PROCESO    RUNTIME    PROYECTO
  ------  -------  ---------  ---------  -----------------------
  3000    12345    node       Node.js    ~/code/frontend
  8080    23456    python3    Python     ~/code/api
  9090    34567    cargo      Rust       ~/code/servicio
```

- Detecta Node.js, Python, Go, Rust, Ruby, Java y más
- Separa servidores dev de servicios del sistema
- Mata por puerto, PID o nombre — interactivo o en scripts

### Limpieza del Sistema — Docker, Cachés, VMs, Logs

Libera espacio en disco de forma segura. Cada elemento muestra su nivel de riesgo antes de eliminar cualquier cosa.

- **Indicadores de riesgo**: seguro (verde) · precaución (amarillo) · peligro (rojo)
- **Modal de confirmación** con explicación por elemento, o limpieza masiva con `[x]`
- **Docker**: imágenes huérfanas, contenedores detenidos, caché de build
- **Cachés**: Homebrew, npm, pip, Cargo, Xcode, CocoaPods, Go, Gradle
- **Logs**: logs de dev >10 MB con más de 7 días
- **VMs**: disco VM de Docker, emuladores Android, VMs antiguas

Seguridad: lista de rutas bloqueadas (`/System`, `/bin`, `/usr`...), verificación de apps activas, filtros por antigüedad, log de operaciones, whitelist, modo dry-run. Inspirado en [tw93/mole](https://github.com/tw93/mole).

### Auditoría de Seguridad — Secretos, OWASP, Dependencias

Escáner de 4 fases para cualquier directorio de proyecto. Define la carpeta con `[a]` en el TUI o pásala como argumento.

1. **Secretos**: API keys (AWS, GitHub, Anthropic, OpenAI, Stripe, Slack), credenciales, archivos `.env`
2. **Historial git**: recorre diffs de commits buscando secretos eliminados posteriormente
3. **OWASP Top 10:2025**: inyección SQL, inyección de comandos, XSS, criptografía insegura, path traversal, deserialización
4. **Dependencias**: API batch de [OSV.dev](https://osv.dev) + `npm audit` para CVEs conocidos

Severidad contextual (código fuente > config > tests > docs). `.sparkauditignore` para suprimir hallazgos revisados.

### Escáner de Certificados — Salud SSL/TLS

```bash
spark certs           # Keychain + directorio home
spark certs --expired # Solo expirados
spark certs --summary # Solo conteos
```

- macOS Keychain: expirados, por expirar, válidos — agrupados por emisor
- Directorio home: archivos `.pem`, `.crt`, `.key`, claves SSH sueltas
- Recomendaciones: Apple (seguro eliminar), Desarrollador (renovar en Xcode), Autofirmado (revisar y rotar)

### Actualizador — Gestor de Herramientas Dev

Rastrea y actualiza 55 herramientas de desarrollo en 8 categorías: IA, terminales, IDEs, infraestructura, runtimes, utilidades, productividad, sistema.

Vista de tabla con versión actual, versión disponible e indicadores de estado. Actualiza una herramienta o todas las desactualizadas de una vez.

---

## CLI

```bash
spark                          # Abrir TUI
spark init                     # Integración de shell, completions, config
spark doctor                   # Validar instalación

# Repos
spark clone <url>              # Clonar (compatible ghq, shorthand owner/repo)
spark clone <url> -p           # Clonar via SSH
spark list [-p] [query]        # Listar repos (árbol: branch + antigüedad + tags)
spark search <query>           # Buscar repos
spark status [query]           # Ver qué repos necesitan pull
spark status --tag <tag>       # Estado filtrado por tag
spark pull <query|all>         # Pull repos (ff-only)
spark pull all --tag <tag>     # Pull repos por tag
spark cd <nombre>              # Imprimir ruta al repo
spark rm <query>               # Eliminar un repo

# Etiquetas
spark tag add <repo> <tag>     # Etiquetar un repo
spark tag remove <repo> <tag>  # Quitar etiqueta
spark tag list [tag]           # Listar etiquetas o repos en un tag
spark tag delete <tag>         # Eliminar una etiqueta
spark tag rename <old> <new>   # Renombrar etiqueta

# Puertos
spark ps                       # Servidores dev (pid, proceso, runtime, proyecto)
spark ps --all                 # Todos los puertos
spark ps <query>               # Buscar procesos por nombre
spark ps --kill <target>       # Matar por puerto, PID o nombre
spark ps <query> --kill        # Matar no-interactivo (exit 0/1 para scripts)

# Auditoría de seguridad
spark audit [ruta]             # Auditoría completa (secretos + OWASP + deps)
spark audit --deps             # Solo dependencias
spark audit --offline          # Sin red
spark audit --init             # Crear .sparkauditignore
spark audit -o reporte.txt     # Guardar reporte en archivo

# Certificados
spark certs                    # Keychain + directorio home
spark certs --keychain         # Solo Keychain
spark certs --expired          # Solo expirados
spark certs --summary          # Solo conteos

# Config
spark root [--set <ruta>]      # Ver/cambiar raíz de repos
spark config [key --set v]     # Ver/actualizar configuración
spark completions <shell>      # Completions zsh/bash/fish
spark agent                    # Tips de integración con agentes IA
spark --dry-run                # Modo preview (sin acciones destructivas)
```

---

## Controles

| Tab | Tecla | Acción |
|-----|-------|--------|
| **Global** | `TAB` | Cambiar tab: Scanner → Repos → Ports → System → Audit → Updater |
| | `q` | Atrás / cerrar modal |
| | `Ctrl+C` | Salir |
| **Scanner** | `ENTER` | Escanear directorio / ver detalle |
| | `a` | Agregar ruta de escaneo |
| | `c` | Limpiar artefactos |
| | `x` | Borrar repo |
| | `s` | Ordenar resultados |
| | `?` | Explicación de calificaciones de salud |
| **Repos** | `ENTER` | Modal de acciones (pull, abrir, borrar) |
| | `c` | Clonar un repo |
| | `u` / `U` | Pull seleccionado / pull todos los atrasados |
| | `r` | Actualizar estados |
| **Ports** | `ENTER` | Modal de acciones (matar, abrir carpeta) |
| | `SPACE` | Seleccionar |
| | `x` / `X` | Matar seleccionado / matar todos los servidores dev |
| **System** | `ENTER` | Modal de detalle/riesgo |
| | `SPACE` | Seleccionar elemento |
| | `x` | Limpiar seleccionados |
| **Audit** | `a` | Definir carpeta a analizar |
| | `ENTER` | Ver detalle de hallazgos |
| | `r` | Re-escanear |
| **Updater** | `u` / `U` | Actualizar seleccionado / actualizar todos |
| | `ENTER` | Ver changelog |

---

## Configuración

```bash
spark config                             # Ver todas las opciones
spark config repos_root --set ~/codigo   # Cambiar raíz de repos
spark root --set ~/codigo                # Equivalente
```

Archivo de configuración: `~/.config/spark/config.toml` (macOS: `~/Library/Application Support/spark/`)

| Archivo | Propósito |
|---------|-----------|
| `config.toml` | Configuración principal |
| `whitelist.txt` | Rutas a proteger durante limpieza del sistema |
| `operations.log` | Log de auditoría de acciones de limpieza |
| `repo_status_cache.json` | Caché de estados de repos (TTL 4h) |

```toml
# ~/.config/spark/config.toml
repos_root = "~/repos"
stale_threshold_days = 90
large_artifact_threshold = 104857600  # 100 MB
use_trash = true
max_scan_depth = 6
```

---

## Para Agentes IA

```bash
spark agent    # Tips de integración
spark ingest   # Generar contexto comprimido para LLMs (via trs)
```

Agrega a tu `CLAUDE.md` o `.cursorrules`:

```
Repos gestionados por spark (compatible con ghq).
Usa `spark cd <nombre>` para encontrar rutas a repos.
Usa `spark root` para obtener la raíz de repos.
Usa `spark list` para ver el árbol completo de repos.
```

---

## Stack Tecnológico

| | |
|---|---|
| Lenguaje | Rust |
| TUI | [Ratatui](https://ratatui.rs) + [crossterm](https://github.com/crossterm-rs/crossterm) |
| Async | [tokio](https://tokio.rs) |
| Git | [git2](https://github.com/rust-lang/git2-rs) (bindings de libgit2) |
| HTTP | [reqwest](https://github.com/seanmonstar/reqwest) + rustls |
| CLI | [clap 4](https://github.com/clap-rs/clap) |
| Binario | ~4 MB (LTO + strip), sin dependencias de runtime |
| Tests | 127 pasando, 0 warnings |

---

## Contribuir

```bash
git clone https://github.com/dPeluChe/spark.git
cd spark
cargo test                  # todos los tests deben pasar
cargo clippy -- -D warnings # cero warnings
cargo fmt -- --check        # formato debe coincidir
```

Lee [CONTRIBUTING.md](CONTRIBUTING.md) para las guías de código, [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) para el mapa del codebase, y [docs/TASK_TODO.md](docs/TASK_TODO.md) para el roadmap.

---

## Créditos

Construido con [Ratatui](https://ratatui.rs), [tokio](https://tokio.rs), [git2](https://github.com/rust-lang/git2-rs), [clap](https://github.com/clap-rs/clap).

Inspirado en [ghq](https://github.com/x-motemen/ghq), [mole](https://github.com/tw93/mole), [lazygit](https://github.com/jesseduffield/lazygit), [k9s](https://k9scli.io).

---

**SPARK** v0.5.1 — Licencia MIT

Un producto de [Iteris](https://iteris.tech) · Publicado y mantenido por [@dPeluChe](https://github.com/dPeluChe)
