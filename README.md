# sexlog

本地私密的性生活记录工具，使用 Rust + SQLite 编写。支持命令行快速记录、交互式录入，以及一个完全离线的静态数据仪表盘。

所有数据仅保存在本机 `~/.sexlog` 目录，不上传、不联网。

## 功能

- 一条命令记录，或根据上次记录自动带默认值的交互式录入
- SQLite 存储，无需额外安装数据库
- 静态 HTML 仪表盘，无外部 CDN、无网络请求
- 指标卡：次数、每周频率、平均满意度、平均时长、平均间隔、措施使用率
- 图表：月度次数、满意度分布、星期/时段分布、时长趋势、伴侣与类型分布、日期热力图、行为标签
- 最近记录表
- 按范围（近 7 天 / 近 1 个月 / 近半年 / 近 1 年 / 全部）、类型、伴侣筛选
- 纯终端 TUI 看板，无需浏览器即可交互式查看同一套统计
- 本地 Web 服务器预览，数据变化后重新生成即可

## 安装

需要 Rust 1.85 或更高版本（2024 edition）。

```bash
git clone git@github.com:jimicat/sexlog.git
cd sexlog
cargo build --release
```

可执行文件位于 `target/release/sexlog`，也可以直接安装到 Cargo 的 bin 目录：

```bash
cargo install --path .
```

### Homebrew（macOS / Linux）

仓库同时作为 Homebrew tap 提供预编译安装：

```bash
brew tap jimicat/sexlog https://github.com/jimicat/sexlog.git
brew trust jimicat/sexlog
brew install sexlog
```

`brew trust` 用于允许加载第三方 tap。升级与卸载：

```bash
brew update
brew upgrade sexlog

brew uninstall sexlog
brew untap jimicat/sexlog
```

## 自动构建与发布

仓库内置 GitHub Actions 工作流 [.github/workflows/release.yml](.github/workflows/release.yml)，推送 `v*` 标签后会自动：

1. 在 5 种平台上编译 release 二进制
2. 打包并生成 SHA-256 校验文件
3. 创建 GitHub Release 并上传全部产物

| 平台 | Rust target | 产物 |
|---|---|---|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| macOS Intel | `x86_64-apple-darwin` | `.tar.gz` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `.tar.gz` |
| Windows x86_64 | `x86_64-pc-windows-msvc` | `.zip` |

产物命名格式为 `sexlog-<版本>-<target>.tar.gz`（Windows 为 `.zip`），压缩包内包含可执行文件、`README.md` 和 `LICENSE`。

发布新版本：

```bash
git tag v0.1.0
git push origin v0.1.0
```

推送普通分支不会创建 Release。也可以在 Actions 页面手动触发 `Release` 工作流，仅构建并保留工作流产物用于验证，不会发布 Release。

> Linux 产物基于 Ubuntu 22.04 构建，需要 glibc 2.35 或更高版本。

发布新版本后，需要同步更新 [Formula/sexlog.rb](Formula/sexlog.rb) 中的 `version` 和四个平台的 `sha256`，Homebrew 才能安装新版本。

## 快速开始

```bash
# 记录一条
sexlog add --date 2026-11-05 --time 22:30 \
  --kind partner --partner A \
  --acts "前戏,插入" --protection yes \
  --orgasms 1 --satisfaction 5 --duration 40

# 查看最近记录
sexlog list

# 生成仪表盘并本地预览
sexlog report
sexlog serve --open

# 直接在终端查看交互式看板
sexlog tui
```

## 命令

### `sexlog add`

单条命令写入记录。

| 参数 | 必填 | 默认值 | 说明 |
|---|---|---|---|
| `--date` | 否 | 今天 | 日期，格式 `YYYY-MM-DD` |
| `--time` | 否 | 当前时间 | 时间，格式 `HH:MM` |
| `--kind` | 否 | `partner` | 类型：`partner` / `solo` / `other` |
| `--partner` | 否 | - | 伴侣名称，最长 64 字符 |
| `--acts` | 否 | - | 行为标签，逗号分隔，可重复传入 |
| `--protection` | 否 | `no` | 是否采取保护措施：`yes` / `no` |
| `--orgasms` | 否 | `1` | 高潮次数，0–20 |
| `--satisfaction` | 是 | - | 满意度，1–5 |
| `--duration` | 是 | - | 时长（分钟），1–1440 |
| `--notes` | 否 | - | 备注，最长 500 字符 |

行为标签支持 `,`、`，`、`、` 分隔，会自动去重，单个标签最长 32 字符、最多 30 个。

### `sexlog log`

交互式逐项录入。日期、类型、伴侣、行为、措施、满意度等字段会自动填入上一条记录的对应值，直接回车即可复用。

### `sexlog list`

```bash
sexlog list --limit 20
```

按时间倒序显示最近记录，默认 15 条，最多 500 条。

### `sexlog report`

读取数据库并生成 `~/.sexlog/report/index.html`。仪表盘是纯静态文件，可以直接用浏览器打开，也可以托管到任意静态服务器。

数据发生变化后需要重新执行 `sexlog report`。

### `sexlog serve`

先重新生成仪表盘，再启动本地 HTTP 服务：

```bash
sexlog serve                 # http://127.0.0.1:8787
sexlog serve --port 9000
sexlog serve --host 0.0.0.0  # 监听非回环地址时会有风险提示
sexlog serve --open          # 启动后自动打开浏览器
```

服务默认只监听 `127.0.0.1`，每次请求都会重新读取报告文件，因此重新生成后刷新页面即可看到新数据。

### `sexlog tui`

在终端中直接打开交互式数据看板，读取同一个 SQLite 数据库，不生成 HTML、不启动服务：

```bash
sexlog tui                          # 默认近 1 年
sexlog tui --range 7d
sexlog tui --range 6m
sexlog tui --range all --kind partner --partner A
sexlog tui --kind solo
```

| 参数 | 必填 | 默认值 | 说明 |
|---|---|---|---|
| `--range` | 否 | `1y` | 统计范围：`7d` / `1m` / `6m` / `1y` / `all` |
| `--kind` | 否 | 全部 | 类型：`partner` / `solo` / `other` |
| `--partner` | 否 | 全部 | 伴侣名称；`__none__` 表示未填写 |

看板顶部有 4 个标签页，底部显示按键提示：

| 按键 | 作用 |
|---|---|
| `q` / `Esc` | 退出 |
| `←` / `→` / `h` / `l` / `Tab` | 切换标签页 |
| `1`–`4` | 直接跳到指定标签页 |
| `r` | 循环切换范围（7d → 1m → 6m → 1y → all） |
| `k` | 循环切换类型 |
| `p` | 循环切换伴侣 |
| `c` | 重置筛选（范围回到近 1 年） |

标签页内容：

- **概览**：6 个指标卡、月度次数柱状图、日期热力图
- **趋势**：最近 30 次时长折线图、月度次数
- **分布**：满意度、伴侣、类型、星期 × 时段、行为标签
- **记录**：最近记录表

TUI 需要交互式终端，建议窗口至少 72×24。若终端尺寸不足会显示提示；在非交互环境（如管道、CI）中运行会直接报错退出。图表使用 ratatui + crossterm 绘制，无需浏览器或其他依赖。

### `sexlog config`

查看当前配置、数据库、仪表盘和服务器地址：

```bash
sexlog config
sexlog config --init   # 写入默认配置，不会覆盖已有文件
```

## 仪表盘

顶部可按以下条件筛选，图表联动更新：

- **范围**：近 7 天 / 近 1 个月 / 近半年 / 近 1 年 / 全部
- **类型**：全部 / 伴侣 / 独自 / 其他
- **伴侣**：全部、指定伴侣或未填写

图表与区块包括：

- 6 个核心指标卡
- 月度次数柱状图
- 满意度分布
- 星期 × 时段分布
- 时长趋势（含平均值参考线）
- 伴侣与类型分布
- 日期热力图（窗口随所选范围联动，全部范围可横向滚动）
- 行为标签统计
- 最近 10 条记录表

仪表盘不加载任何外部资源，所有数据通过内嵌 JSON 传入，页面内不对用户数据使用 `innerHTML`。

## 数据与配置

默认目录结构：

```text
~/.sexlog/
├── config.toml       # 可选配置文件
├── sexlog.db         # SQLite 数据库
└── report/
    └── index.html    # 静态仪表盘
```

目录权限为 `700`，配置、数据库和报告文件权限为 `600`。

### 配置文件

运行 `sexlog config --init` 生成 `~/.sexlog/config.toml`：

```toml
# 数据库文件路径
# database = "sexlog.db"

# 静态仪表盘输出目录
# report_dir = "report"

[server]
host = "127.0.0.1"
port = 8787
```

- 相对路径以配置文件所在目录为基准
- 可自定义 `database`、`report_dir`、`server.host`、`server.port`

### 路径覆盖

| 用途 | 方式 |
|---|---|
| 数据库 | `--db <PATH>` 或环境变量 `SEXLOG_DB` |
| 配置文件 | 环境变量 `SEXLOG_CONFIG` |

命令行参数优先级最高。

### 数据字段

| 字段 | 类型 | 说明 |
|---|---|---|
| `occurred_at` | TEXT | 发生时间，本地时间 `YYYY-MM-DD HH:MM` |
| `kind` | TEXT | `partner` / `solo` / `other` |
| `partner` | TEXT | 伴侣名称，可空 |
| `acts` | TEXT | 行为标签，逗号分隔存储 |
| `protection` | INTEGER | 是否采取保护措施，0 / 1 |
| `orgasms` | INTEGER | 高潮次数，0–20 |
| `satisfaction` | INTEGER | 满意度，1–5 |
| `duration_min` | INTEGER | 时长（分钟），1–1440 |
| `notes` | TEXT | 备注，可空 |
| `created_at` | TEXT | 记录创建时间，UTC RFC3339 |

## 隐私与安全

- 数据完全保存在本机，不发起任何外部网络请求
- SQLite 开启 `secure_delete`，删除的记录内容会被覆写
- 数据库与报告文件权限 `600`，数据目录权限 `700`
- Web 服务默认仅监听回环地址，并设置 CSP、`nosniff`、`no-referrer`、`no-store` 等响应头
- 仪表盘内嵌数据会转义 `<`、`>`、`&`，避免注入

## 开发

```bash
cargo fmt
cargo check
cargo clippy --all-targets
cargo build
```

## License

[MIT](LICENSE)
