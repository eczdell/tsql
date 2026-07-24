# tsql — PostgreSQL Terminal Visualizer & Interactive ER Diagram Explorer 🚀

`tsql` is an ultra-fast, modern, interactive Terminal User Interface (TUI) for **PostgreSQL** built with Rust, `ratatui`, `tokio-postgres`, and `nucleo-matcher`. Inspired by tools like TablePlus and DataGrip, `tsql` provides instant schema inspection, interactive ER relationship diagrams with 5-segment bypass routing, 2D proximity hop navigation, relational foreign key jumping, SIMD fuzzy table search, and memory-efficient connection management directly inside your terminal.

---

## ✨ Features

- **⚡ Blazing Fast & Memory Efficient**: Built in async Rust using `tokio` and `tokio-postgres`. Includes in-memory schema caching (`HashMap`) for zero network latency when navigating back and forth across tables.
- **🗺️ Interactive Relationships ER Diagram Visualizer**:
  - **2D Proximity Hop Navigation (`hjkl` / Arrow Keys)**: Move focus across 2D canvas columns seamlessly.
  - **Focused Relationship View**: Highlights active table relationships in bright gold/yellow to eliminate visual clutter. Toggle all lines with `a`.
  - **5-Segment Orthogonal Bypass Routing**: Connection lines between non-adjacent columns route cleanly around table cards, avoiding all box overlaps.
  - **Collision-Free Channels & Margins**: 2-cell empty padding between cards and lines ensures zero track collisions.
  - **Density Scaling (`i` / `o`)**: Zoom in/out without hiding fields—all table columns remain fully visible while density scales.
  - **Smooth Panning & Scrolling (`Ctrl+d` / `Ctrl+u`)**: Vertical canvas scrolling with auto-centering on focused entities.
  - **Column Repositioning (`r`)**: Instantly shuffle/reposition table distribution to untangle complex schemas.
- **🔍 SIMD Fuzzy Table Search**: Press `/` to trigger instant fuzzy filtering powered by `nucleo-matcher` (the same SIMD engine used in Helix & fzf-native).
- **🔗 Relational Foreign Key Jumps**: Interactive green cell cursor `▶[ value ]◀`. Press `Enter` on any foreign key column (`🔗 FK`) or polymorphic ID (`entity_id`, `_id`) to **instantly jump to the linked table**.
- **🍞 Multi-Level Navbar Breadcrumbs & Back/Forward Navigation**: Displays your relational navigation history chain (`[audit_logs ➔ •cases• ➔ case_subtypes]`). Press `b` to step **Back** and `f` to step **Forward** (just like Google Chrome!).
- **🖥️ Fullscreen Mode (`m` / `z`)**: Expand any table grid or ER diagram to fill your entire terminal screen.
- **🔑 PK / FK Header Badges**: Color-coded badges for Primary Keys (`[pk]` in Light Green), Foreign Keys (`[fk]` in Light Cyan), and Composite Keys (`[fk/pk]` in Light Magenta).
- **🔌 Multi-Server Connection Manager**: Add, edit, delete, and switch PostgreSQL server connections on the fly (`~/.config/tsql/config.toml`).
- **🎯 Database & User Inspection**: View all databases, check sizes/collations, view database roles/superusers, and switch active databases dynamically.
- **💻 Interactive SQL Query Runner**: Write and run custom SQL queries directly inside `tsql`.

---

## ⌨️ Keyboard Shortcuts & Navigation

### Global Shortcuts
| Key | Action |
| --- | --- |
| `1` - `6` | Switch Tabs (`Tables`, `Databases`, `Users`, `Query Runner`, `Connections`, `Relationships`) |
| `Tab` | Toggle Focus between Table List / Sidebar and Main Content Area |
| `/` | Open SIMD Fuzzy Table Filter |
| `s` | Toggle system tables (`information_schema`, `pg_catalog`) |
| `r` | Refresh database tables & clear caches |
| `q` or `Esc` | Quit / Close modal |

### Relational Data Grid & Breadcrumbs (Tab 1)
| Key | Action |
| --- | --- |
| `Enter` | **Jump to Relational Foreign Key Table** (when on `🔗 FK` column) |
| `j` / `k` (or `Up` / `Down`) | Move grid cursor up & down |
| `h` / `l` (or `Left` / `Right`) | Move grid cursor left & right |
| `b` | **Step Back** along breadcrumb navigation trail |
| `f` | **Step Forward** along breadcrumb navigation trail |
| `m` or `z` | **Toggle Fullscreen Mode** |
| `+` or `=` | **Zoom In Grid** (Widen column widths) |
| `-` or `_` | **Zoom Out Grid** (Narrow column widths) |
| `n` / `p` | Next Page / Previous Page (50 rows/page) |

### Interactive ER Diagram Visualizer (Tab 6)
| Key | Action |
| --- | --- |
| `h` / `j` / `k` / `l` | **2D Proximity Navigation** (Move focus to nearest table box in 2D space) |
| `Ctrl+d` | **Scroll Canvas Down** (10 rows) |
| `Ctrl+u` | **Scroll Canvas Up** (10 rows) |
| `i` | **Zoom In** (Expand card width and channel spacing) |
| `o` | **Zoom Out** (Pack cards and spacing denser, keeping all fields visible) |
| `a` | **Toggle All Lines** (Switch between Focused Table View and All Relationships View) |
| `r` | **Reposition Layout** (Shuffle column assignment to untangle complex connections) |
| `Enter` | **View Table Data Grid** for focused entity |

### Connection Manager (Tab 5)
| Key | Action |
| --- | --- |
| `j` / `k` | Navigate connection profiles |
| `a` | **Add New Connection Profile** (step-by-step form) |
| `e` or `u` | **Edit / Update Selected Connection Profile** |
| `d` or `Delete` | **Delete Selected Connection Profile** |
| `Enter` or `c` | **Connect to Selected Server Profile** |

---

## 🛠️ Cross-Platform Installation Guide

`tsql` supports **Linux**, **macOS**, and **Windows** (Windows Terminal / PowerShell).

### Prerequisites
- **Rust Toolchain** (1.70+): Install via [rustup.rs](https://rustup.rs/)

---

### 🐧 Linux & 🍎 macOS Installation

#### Option 1: Install User-Local (Recommended)
Installs `tsql` directly to `~/.local/bin/tsql`:
```bash
git clone https://github.com/eczdell/tsql.git
cd tsql
make install-local
```
> **Note**: Ensure `~/.local/bin` is in your `$PATH` (e.g. `export PATH="$HOME/.local/bin:$PATH"` in `~/.bashrc` or `~/.zshrc`).

#### Option 2: Install System-Wide (Global)
Installs `tsql` to `/usr/local/bin/tsql`:
```bash
sudo make install
```

#### Option 3: Install via Cargo
Installs `tsql` to `~/.cargo/bin/tsql`:
```bash
cargo install --path .
```
or using Makefile:
```bash
make install-cargo
```

---

### 🪟 Windows Installation (PowerShell / Command Prompt)

#### Step 1: Open PowerShell or Command Prompt
Clone the repository:
```powershell
git clone https://github.com/eczdell/tsql.git
cd tsql
```

#### Step 2: Build & Install via Cargo
```powershell
cargo install --path .
```
This builds the release binary and installs `tsql.exe` into `%USERPROFILE%\.cargo\bin\tsql.exe`.

#### Step 3: Ensure Cargo Bin is in Environment PATH
Make sure `%USERPROFILE%\.cargo\bin` is added to your User Environment Variables PATH.

Now run in PowerShell or Windows Terminal:
```powershell
tsql
```

---

## ⚙️ Configuration

`tsql` stores connection settings in `~/.config/tsql/config.toml` (Linux/macOS) or `%APPDATA%\tsql\config.toml` (Windows).

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
