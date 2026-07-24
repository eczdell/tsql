# tsql — PostgreSQL Terminal Visualizer & Schema Explorer 🚀

`tsql` is an ultra-fast, modern, interactive Terminal User Interface (TUI) for **PostgreSQL** built with Rust, `ratatui`, `tokio-postgres`, and `nucleo-matcher`. Inspired by tools like `@[gitscope]`, `tsql` provides instant database inspection, relational foreign key jumping, SIMD fuzzy table search, and memory-efficient connection management directly inside your terminal.

---

## ✨ Features

- **⚡ Blazing Fast & Memory Efficient**: Built in async Rust using `tokio` and `tokio-postgres`. Includes in-memory schema caching (`HashMap`) for zero network latency when navigating back and forth across tables.
- **🔍 SIMD Fuzzy Table Search**: Press `/` to trigger instant fuzzy filtering powered by `nucleo-matcher` (the same SIMD engine used in Helix & fzf-native).
- **🔗 Relational Foreign Key Jumps**: Interactive green cell cursor `▶[ value ]◀`. Press `Enter` on any foreign key column (`🔗 FK`) or polymorphic ID (`entity_id`, `_id`) to **instantly jump to the linked table**.
- **🍞 Multi-Level Navbar Breadcrumbs & Back/Forward Navigation**: Displays your relational navigation history chain (`[audit_logs ➔ •cases• ➔ case_subtypes]`). Press `b` to step **Back** and `f` to step **Forward** (just like Google Chrome!).
- **🖥️ Fullscreen Mode (`m` / `z`)**: Expand any table grid to fill your entire terminal screen.
- **🔍 Grid Zoom In & Zoom Out (`+` / `-`)**: Press `+` to widen columns and view long UUIDs, text, or JSON fields without truncation. Press `-` to narrow columns and fit more data on screen.
- **🔑 PK / FK Header Badges**: Visual indicators for Primary Keys (`🔑 PK`), Foreign Keys (`🔗 FK`), and Composite Keys (`🔑🔑 PK/FK`).
- **🔌 Multi-Server Connection Manager**: Add, edit, delete, and switch PostgreSQL server connections on the fly (`~/.config/tsql/config.toml`).
- **🎯 Database & User Inspection**: View all databases, check sizes/collations, view database roles/superusers, and switch active databases dynamically.
- **💻 Interactive SQL Query Runner**: Write and run custom SQL queries directly inside `tsql`.

---

## ⌨️ Keyboard Shortcuts & Navigation

### General & Navigation
| Key | Action |
| --- | --- |
| `1` - `5` | Switch Tabs (`Tables`, `Databases`, `Users`, `Query Runner`, `Connections`) |
| `Tab` | Toggle Focus between Table List and Data View Grid |
| `j` / `k` (or `Up` / `Down`) | Navigate lists / Move grid cursor up & down |
| `h` / `l` (or `Left` / `Right`) | Move grid cursor left & right |
| `/` | Open SIMD Fuzzy Table Filter |
| `s` | Toggle system tables (`information_schema`, `pg_catalog`) |
| `r` | Refresh database tables & clear caches |
| `q` or `Esc` | Quit / Close modal |

### Relational Data Grid & Breadcrumbs
| Key | Action |
| --- | --- |
| `Enter` | **Jump to Relational Foreign Key Table** (when on `🔗 FK` column) |
| `b` | **Step Back** along breadcrumb navigation trail |
| `f` | **Step Forward** along breadcrumb navigation trail |
| `m` or `z` | **Toggle Fullscreen Mode** |
| `+` or `=` | **Zoom In** (Widen column widths) |
| `-` or `_` | **Zoom Out** (Narrow column widths) |
| `n` / `p` | Next Page / Previous Page (50 rows/page) |

### Connection Manager (Tab 5)
| Key | Action |
| --- | --- |
| `j` / `k` | Navigate connection profiles |
| `a` | **Add New Connection Profile** (step-by-step form) |
| `d` or `Delete` | **Delete Selected Connection Profile** |
| `Enter` or `c` | **Connect to Selected Server Profile** |

---

## 🛠️ Installation & Building

### Prerequisites
- **Rust toolchain** (cargo 1.70+)

### Building from Source

```bash
git clone git@github.com:eczdell/tsql.git
cd tsql
make build
```

### Installing Locally
Install `tsql` directly to your local binaries (`~/.local/bin/tsql`):

```bash
make install-local
```

Make sure `~/.local/bin` is in your `$PATH`. Then run:
```bash
tsql
```

---

## ⚙️ Configuration

`tsql` stores connection settings in `~/.config/tsql/config.toml`.

Example configuration:

```toml
default_connection = "local"

[[connections]]
name = "local"
host = "127.0.0.1"
port = 5432
user = "postgres"
password = "mysecretpassword"
dbname = "postgres"
```

---

## 📄 License

MIT License. Open source and free to use!
