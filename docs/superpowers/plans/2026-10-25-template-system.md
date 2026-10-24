# Framely 模板体系实施计划（Template System）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把模板从 `match` 分支改为 `TemplateSpec` 数据 + 单一 `layout()` 引擎，在保持现有 4 个模板逐像素不变的前提下扩充到 8 个模板，并统一为一套按模板适配的控件集。

**Architecture:** 每个模板 = 一份 `TemplateSpec`（条位/四边边距/条内结构/分隔符/标签比例）+ 一份默认 `ThemeConfig`。`layout()` 按 spec 分派到 4 个条内结构函数，各自返回既有 `Vec<DrawOp>`（不新增 op 变体），`render::compose` 完全不改。

**Tech Stack:** Rust 2024，`framely-core`（零 GUI 依赖）+ `framely-app`（eframe/egui），serde JSON 预设，`image` PNG，`sha2`（仅 dev-dependency，golden 用）。

**Spec:** `docs/superpowers/specs/2026-10-25-template-system-design.md`

## Global Constraints

- 现有 4 个模板（White Studio / Black Bar / Minimal / Magazine）外观**逐像素不变**，由 golden 测试锁定（spec §5、§7）。
- 新增 `ThemeConfig` 字段一律 `#[serde(default)]`（边距用「缺省哨兵」实现兼容，见 Task 2），旧预设无需迁移即可载入（spec §4）。
- 旧版程序读新预设不报错：保存时四边全写出 + `border_ratio` = 四者均值（spec §4.1）。
- 不新增错误变体；非法几何输入由 `.max(1.0)` / `.clamp()` 吸收（spec §8）。
- `render::compose`、`DrawOp`、jpeg 导出、EXIF 策略均**不改**（spec §3.2、§10）。
- 框架内文字始终英文；UI 文案双语，中文经 `i18n`。
- `cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --check` 必须绿。
- 输出为 v0.3.0，不动 v0.1.0/v0.2.0。

## 落地判断（对设计稿歧义的裁决，写入账本）

设计稿内部有三处张力，本计划按下述裁决执行（spec 架构章节 §3.2 优先于 §5 措辞）：

1. **TwoColumn 机型行的"Medium weight"**：§3.2 明令 `DrawOp` 不新增变体、`render::compose` 完全不改，而当前 render 只加载 `FONT_REGULAR`、`Text` op 无字重字段——无法在不改 render 的前提下用 Inter-Medium。裁决：机型行改用**更大字号**表达层级（机型 = 标称字号，参数行 = `0.85×`），而非字重。代价：与 §5 表格「Medium」措辞不符，但符合「render 不改」的硬约束。
2. **Caption Card（Left 条）的"条厚"**：§3.2 说「条所在那条边额外加条厚」，但 §6.2 又对 Caption Card **隐藏条高滑块**，§7 测试要求文字「落在左边距 rect 内」。裁决：Left 条时参数区即左边距本身（`margin_left` 已加宽到 6%），canvas_w = photo + margin_left + margin_right，**不额外加条厚**，`bar_ratio` 对 Caption Card 不生效。
3. **分隔符宽度**：`estimate_width` 已覆盖 `·`/`|`/空格三字符的估宽，fit 路径无需改；仅 `join()` 改为按 `theme.separator` 选串。
4. **StackedLabeled 的字号来源**：若沿用 `font = bar_h * font_scale`（bar_h=条宽），值字号约为条宽一半，任何值串都放不下、默认就丢光字段。裁决：StackedLabeled 的字号按**条宽**推导：`value_size = clamp(round(avail_w * theme.font_scale * 0.5), 9, ..)`，条内边距 `inner_pad = round(strip_w * 0.1)`；`font_scale` 滑块对 Caption Card 隐藏（与 §6.2 一致）。代价：spec 未写公式，golden 不覆盖新模板，以 §7 行为测试（不出界、丢字段不溢出）验证。
5. **`Field::Camera` 默认启用范围**：`from_bytes` 会把机身 `Model` 作为 `Field::Camera` 放进 `parts`。若默认 `enabled` 含 Camera，白棚等旧模板预览会多出机型、违反「现有 4 模板外观不变」。裁决：`FramelyApp::default().enabled` 保持 5 字段（不含 Camera、Date 也不动）；仅当用户切到 **Two-Column** 时 GUI 自动把 Camera 加进该会话 enabled（spec §5 备注「Field::Camera 默认开」仅指该模板）。批量导出沿用既有「全字段、不过滤 enabled」行为（`batch.rs:48-51`），故批量结果会自然多出机型——属新字段涌现，不改既有批量字段（加过滤反而会让 Date 从批量消失，是回归）。
6. **`show_labels` 对非堆叠结构无效**：§6.2 把标签开关标为「全部」适用，但单行/双行条没有标签行可放。裁决：引擎只在 `StackedLabeled` 结构消费 `show_labels`/`label_ratio`；GUI 对全部模板显示开关（遵守控件表），非堆叠模板悬停提示"仅侧边签条生效"。
7. **`center_group` 对 TwoColumn**：§6.2 把「整体居中」列为除 Caption Card 外通用。裁决：引擎对 `TwoColumn` 同样遵守 `center_group`——置真时把 logo+机型栏+参数栏作为一个整体水平居中（复用 `bar_single` 的 optically-centered 数学），否则机型左锚、参数右锚。代价：需实测居中块宽；实现放在 `bar_two_column` 内。
8. **Polaroid 的 `bar_ratio` 默认值**：spec §5 标 Polaroid `bar_ratio` 0.035。`default_for(Polaroid)` 用 0.035；`ThemeConfig` 字段本身 `bar_ratio` 默认仍走 base（0.06）——新模板 `default_for` 显式覆盖。

## Review Focus

设计稿隐含但各任务测试未必覆盖、最可能伤到真实用户的点（每条在 owning task 里补一条钉死它的测试）：

- **旧预设载入**：仅含 `border_ratio`、无 `margin_*` 的 JSON → 四边须等于 `border_ratio`，外观与 v0.2.0 一致（Task 2 测试）。
- **新预设被旧版读**：保存的 JSON 必须仍含合法 `border_ratio` 标量（Task 2 测试 roundtrip 后 `border_ratio ≈ mean`）。
- **TwoColumn 机型为空**：退化为 SingleLine 形状，仅 1 个 Text op，不画空机型栏（Task 4 测试）。
- **Caption Card 字段溢出**：窄条装不下全部堆叠组时按 `FIELD_DROP_ORDER` 丢字段，绝不越出左边距 rect（Task 4 测试）。
- **分隔符换竖线不吞字**：`Separator::Bar` 时估宽计入 `|`，整行仍完整（Task 3/4 测试）。

---

## 文件结构

- `core/src/template.rs` — 加 `BarEdge` / `BarStructure` / `Separator` / `TemplateSpec`；`TemplateId` 扩 8 变体 + `spec()`；`Field` 加 `Camera`；`ThemeConfig` 加边距/条位/分隔符/标签字段 + 兼容方法 + 手写 Hash 扩展。
- `core/src/layout.rs` — `layout()` 重写为 spec 驱动；4 个条内结构函数；`join` 按分隔符。
- `core/tests/golden.rs` — 新建，golden 基准（PNG sha256）。
- `core/tests/fixtures/golden.sha256` — 新建，基准哈希。
- `core/tests/layout.rs` — 扩测：新模板几何、分隔符、标签、边距缩放、出界扩展到 8 模板。
- `core/tests/template.rs` — 扩测：serde 兼容、新变体 kebab id、Camera 进 drop order。
- `app/src/panels.rs` — 控件集 + 按模板可见性。
- `app/src/i18n.rs` — `template_name` 加 4 新模板中文；`field_label` 加 Camera=「机身」。
- `app/src/app.rs` — `FramelyApp::default()` 默认 `enabled` 列表；保存前 `normalize_for_save`。
- `README.md` / `README.zh-CN.md` — 模板表更新为 8 个。
- `docs/superpowers/specs/2026-10-25-template-system-design.md` — 状态改「已实施」。

---

## Task 1: Golden 基准护栏（锁定旧行为）

**Files:**
- Modify: `core/Cargo.toml`（加 dev-dependency `sha2`）
- Create: `core/tests/golden.rs`
- Create: `core/tests/fixtures/golden.sha256`（由本任务生成脚本写出后入库）

**Interfaces:**
- Consumes: 现有 `layout::layout`、`render::compose`、`ThemeConfig::default_for`（当前实现，未改）
- Produces: 一个测试，渲染固定用例矩阵并比对 `golden.sha256`；哈希生成入口 `#[test] #[ignore] fn record_golden()`（设环境变量触发写文件）。

- [ ] **Step 1: 加 dev-dependency**

`core/Cargo.toml` 的 `[dev-dependencies]` 下加：
```toml
sha2 = "0.10"
```

- [ ] **Step 2: 写 golden 测试（含 record 模式）**

创建 `core/tests/golden.rs`。矩阵 = 4 旧模板 × 3 画幅（2000×1333 / 1333×2000 / 1600×1600）× 3 品牌（Sony/Leica/Unknown）× 2 logo 模式（Mono/Color）。每个用例：合成 `RgbaImage` → PNG 编码 → sha256 hex → 按 key 排序写/比对。用例字段用固定 `Fields`（写死字符串，不依赖 EXIF）。

```rust
use framely_core::assets::LogoMode;
use framely_core::brand::Brand;
use framely_core::layout::layout;
use framely_core::render::compose;
use framely_core::template::{Field, Fields, TemplateId, ThemeConfig};
use sha2::{Digest, Sha256};

fn fields() -> Fields {
    Fields {
        parts: vec![
            (Field::Lens, "FE 35mm F1.8".into()),
            (Field::Aperture, "f/1.8".into()),
            (Field::Shutter, "1/250".into()),
            (Field::Iso, "ISO100".into()),
        ],
        custom_line: None,
    }
}

fn cases() -> Vec<(String, u32, u32, TemplateId, Brand, LogoMode)> {
    let sizes = [(2000u32, 1333u32), (1333, 2000), (1600, 1600)];
    let tpls = [
        TemplateId::WhiteStudio,
        TemplateId::BlackBar,
        TemplateId::Minimal,
        TemplateId::Magazine,
    ];
    let brands = [Brand::Sony, Brand::Leica, Brand::Unknown];
    let modes = [LogoMode::Mono, LogoMode::Color];
    let mut v = Vec::new();
    for &(w, h) in &sizes {
        for &t in &tpls {
            for &b in &brands {
                for &m in &modes {
                    v.push((format!("{t:?}-{w}x{h}-{b:?}-{m:?}"), w, h, t, b, m));
                }
            }
        }
    }
    v
}

fn png_hash(w: u32, h: u32, tpl: TemplateId, brand: Brand, mode: LogoMode) -> String {
    let photo = image::RgbaImage::from_pixel(w, h, image::Rgba([120, 130, 140, 255]));
    let theme = ThemeConfig::default_for(tpl);
    let l = layout(tpl, w, h, brand, 2.0, &theme, &fields());
    let canvas = compose(&photo, &l, mode).unwrap();
    let mut png = Vec::new();
    canvas.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    let mut d = Sha256::new();
    d.update(&png);
    hex(&d.finalize())
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
#[ignore = "run with FRAMELY_GOLDEN=record to (re)write baselines"]
fn record_golden() {
    // Only re-run at the pre-rewrite baseline commit to capture references.
    let mut out = String::new();
    for (key, w, h, t, b, m) in cases() {
        out.push_str(&format!("{key}\t{}\n", png_hash(w, h, t, b, m)));
    }
    let path = format!("{}/tests/fixtures/golden.sha256", env!("CARGO_MANIFEST_DIR"));
    std::fs::write(&path, out).unwrap();
}

#[test]
fn golden_matches_baseline() {
    let path = format!("{}/tests/fixtures/golden.sha256", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).expect("run record_golden at baseline to create golden.sha256");
    let want: std::collections::BTreeMap<String, String> = text
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    for (key, w, h, t, b, m) in cases() {
        let got = png_hash(w, h, t, b, m);
        assert_eq!(got, want[&key], "golden mismatch for {key}");
    }
}
```

- [ ] **Step 3: 在基线代码上生成基准（此时 layout 尚未改，输出即旧行为）**

Run: `FRAMELY_GOLDEN=record cargo test -p framely-core --test golden -- --ignored record_golden`
Expected: PASS，生成 `core/tests/fixtures/golden.sha256`（72 行）。

- [ ] **Step 4: 立即校验基准自洽**

Run: `cargo test -p framely-core --test golden`
Expected: PASS（72 用例哈希全等）。

- [ ] **Step 5: 提交**

```bash
git add core/Cargo.toml core/tests/golden.rs core/tests/fixtures/golden.sha256
git commit -m "test: add golden fixtures locking the 4 existing templates"
```

---

## Task 2: `TemplateSpec` / 枚举 / `ThemeConfig` 扩展 + serde 兼容

旧模板此刻仍走旧 layout 代码（本任务不碰 layout.rs），故 golden 必须保持绿。

**Files:**
- Modify: `core/src/template.rs`
- Modify: `core/src/layout.rs`（仅加穷尽 match 的过渡臂，Task 3 整体重写时删除）
- Modify: `core/tests/template.rs`

**Interfaces:**
- Produces:
  - `pub enum BarEdge { Bottom, Top, Left }`（`Clone Copy Debug PartialEq Eq Hash Serialize Deserialize`，`#[serde(rename_all="kebab-case")]`）
  - `pub enum BarStructure { SingleLine, TwoLine, TwoColumn, StackedLabeled }`（同派生）
  - `pub enum Separator { Dot, Bar, Space }`（同派生）
  - `pub struct TemplateSpec { id, display_name, bar_edge, margins:[f64;4], structure, separator, label_ratio }`（`Debug`）
  - `impl TemplateId { pub fn spec(self) -> &'static TemplateSpec; }`
  - `Field::Camera` 变体 + `id()`="camera" + `label()`="Camera"
  - `ThemeConfig` 新字段：`margin_top/bottom/left/right: f64`（serde default = 哨兵 -1.0）、`bar_position: BarEdge`（serde default）、`separator: Separator`、`show_labels: bool`、`label_ratio: f64`；`border_ratio` 保留
  - `impl ThemeConfig { pub fn effective_margins(&self) -> [f64;4]; pub fn normalize_for_save(&mut self); pub fn separator_str(sep: Separator) -> &'static str; }`
  - `FIELD_DROP_ORDER` 扩为含 `Camera`（排在 `Date` 之后）

- [ ] **Step 1: 写失败测试（serde 兼容 + kebab id + Camera drop order）**

在 `core/tests/template.rs` 追加：

```rust
#[test]
fn old_preset_without_margins_uses_border_ratio() {
    let json = r#"{"border_ratio":0.05,"bar_ratio":0.06,"font_scale":0.45,
        "logo_scale":0.6,"background":[255,255,255,255],
        "foreground":[26,26,26,255],"part_spacing":0.8}"#;
    let cfg: ThemeConfig = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.effective_margins(), [0.05, 0.05, 0.05, 0.05]);
}

#[test]
fn new_preset_roundtrips_with_asymmetric_margins() {
    let mut cfg = ThemeConfig::default_for(TemplateId::Polaroid);
    cfg.margin_top = 0.03;
    cfg.margin_bottom = 0.14;
    cfg.normalize_for_save();
    let json = serde_json::to_string(&cfg).unwrap();
    let back: ThemeConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.effective_margins(), [0.03, 0.14, 0.03, 0.03]);
    // Old versions must still read a sane scalar border.
    assert!((back.border_ratio - (0.03 + 0.14 + 0.03 + 0.03) / 4.0).abs() < 1e-9);
}

#[test]
fn new_template_ids_serialize_kebab_case() {
    for (t, id) in [
        (TemplateId::Polaroid, "polaroid"),
        (TemplateId::TwoColumn, "two-column"),
        (TemplateId::TopBar, "top-bar"),
        (TemplateId::CaptionCard, "caption-card"),
    ] {
        assert_eq!(serde_json::to_string(&t).unwrap(), format!("\"{id}\""));
        assert_eq!(serde_json::from_str::<TemplateId>(&format!("\"{id}\"")).unwrap(), t);
    }
}

#[test]
fn camera_field_is_in_drop_order_after_date() {
    let order = &framely_core::template::FIELD_DROP_ORDER;
    let di = order.iter().position(|f| *f == Field::Date).unwrap();
    let ci = order.iter().position(|f| *f == Field::Camera).unwrap();
    assert_eq!(ci, di + 1);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p framely-core --test template`
Expected: 编译失败（`Polaroid`/`TwoColumn`/`effective_margins` 等未定义）。

- [ ] **Step 3: 实现枚举 + Camera 字段**

在 `core/src/template.rs` 顶部 `TemplateId` 之前加入三个枚举；扩展 `TemplateId`：加 4 变体、`ALL: [TemplateId; 8]`、`id()`/`display_name()` 各加 4 臂；`Field` 加 `Camera`（`id`/`label`）；`FIELD_DROP_ORDER: [Field; 7]` = `[Date, Camera, Lens, Focal, Aperture, Shutter, Iso]`。

`TemplateSpec` + `TemplateId::spec()`（8 条数据，值取自 spec §5 表；margin 用小数分数）：

```rust
pub struct TemplateSpec {
    pub id: TemplateId,
    pub display_name: &'static str,
    pub bar_edge: BarEdge,
    pub margins: [f64; 4], // top, bottom, left, right
    pub structure: BarStructure,
    pub separator: Separator,
    pub label_ratio: f64,
}

impl TemplateId {
    pub fn spec(self) -> &'static TemplateSpec {
        use BarEdge::*; use BarStructure::*; use Separator::*;
        match self {
            TemplateId::WhiteStudio => &TemplateSpec { id: self, display_name: "White Studio", bar_edge: Bottom, margins: [0.04,0.04,0.04,0.04], structure: SingleLine, separator: Dot, label_ratio: 0.6 },
            TemplateId::BlackBar    => &TemplateSpec { id: self, display_name: "Black Bar",    bar_edge: Bottom, margins: [0.04,0.04,0.04,0.04], structure: SingleLine, separator: Dot, label_ratio: 0.6 },
            TemplateId::Minimal     => &TemplateSpec { id: self, display_name: "Minimal",      bar_edge: Bottom, margins: [0.015,0.015,0.015,0.015], structure: SingleLine, separator: Dot, label_ratio: 0.6 },
            TemplateId::Magazine    => &TemplateSpec { id: self, display_name: "Magazine",     bar_edge: Bottom, margins: [0.08,0.08,0.08,0.08], structure: TwoLine, separator: Dot, label_ratio: 0.6 },
            TemplateId::Polaroid    => &TemplateSpec { id: self, display_name: "Polaroid",     bar_edge: Bottom, margins: [0.03,0.14,0.03,0.03], structure: SingleLine, separator: Dot, label_ratio: 0.6 },
            TemplateId::TwoColumn   => &TemplateSpec { id: self, display_name: "Two-Column",   bar_edge: Bottom, margins: [0.04,0.04,0.04,0.04], structure: TwoColumn, separator: Dot, label_ratio: 0.6 },
            TemplateId::TopBar      => &TemplateSpec { id: self, display_name: "Top Bar",      bar_edge: Top,    margins: [0.04,0.06,0.04,0.04], structure: SingleLine, separator: Dot, label_ratio: 0.6 },
            TemplateId::CaptionCard => &TemplateSpec { id: self, display_name: "Caption Card", bar_edge: Left,   margins: [0.03,0.03,0.06,0.03], structure: StackedLabeled, separator: Dot, label_ratio: 0.62 },
        }
    }
}
```

> 注：`&TemplateSpec{..}` 需要 `const`/`'static`。用 `Box::leak` 不合适（每次调用重复分配）。改为每个分支引用一个 `const` 静态，或返回 `&'static` 的最简写法：把 8 个 spec 定义为模块级 `const` 关联常量。实现示例：
> ```rust
> const WHITE_STUDIO: TemplateSpec = TemplateSpec { id: TemplateId::WhiteStudio, display_name: "White Studio", bar_edge: BarEdge::Bottom, margins: [0.04,0.04,0.04,0.04], structure: BarStructure::SingleLine, separator: Separator::Dot, label_ratio: 0.6 };
> // ... 其余 7 个同样 const ...
> pub fn spec(self) -> &'static TemplateSpec {
>     match self {
>         TemplateId::WhiteStudio => &WHITE_STUDIO,
>         /* ... */
>     }
> }
> ```
> `display_name` 字段仍保留（与 `TemplateId::display_name()` 一致，供未来外部数据化）。`TemplateId::display_name()` 现有方法保留不变（返回同名字符串），`id()` 加 4 臂返回 kebab 串。

- [ ] **Step 4: 实现 `ThemeConfig` 扩展 + 兼容方法 + default_for**

新字段全部加 `#[serde(default = "...")]`（边距哨兵函数 `fn unset() -> f64 { -1.0 }`，其余用类型默认）。手写 `Hash` 纳入四个 `margin_*`、`bar_position`(判别值)、`separator`(判别值)、`show_labels`、`label_ratio`（`bar_ratio` 等保留）。

```rust
const MARGIN_UNSET: f64 = -1.0;

pub fn effective_margins(&self) -> [f64; 4] {
    if self.margin_top < 0.0
        && self.margin_bottom < 0.0
        && self.margin_left < 0.0
        && self.margin_right < 0.0
    {
        [self.border_ratio; 4]
    } else {
        [
            self.margin_top.max(0.0),
            self.margin_bottom.max(0.0),
            self.margin_left.max(0.0),
            self.margin_right.max(0.0),
        ]
    }
}

pub fn normalize_for_save(&mut self) {
    let m = self.effective_margins();
    self.margin_top = m[0];
    self.margin_bottom = m[1];
    self.margin_left = m[2];
    self.margin_right = m[3];
    self.border_ratio = (m[0] + m[1] + m[2] + m[3]) / 4.0;
}

pub fn separator_str(sep: Separator) -> &'static str {
    match sep {
        Separator::Dot => " · ",
        Separator::Bar => " | ",
        Separator::Space => " ",
    }
}
```

`default_for(tpl)` 重写为：`base` 之后，四个旧模板沿用原 `border_ratio`（并显式把 `margin_*` 设为同值，避免哨兵触发 layout 差异）、其余参数不变；4 个新模板按 spec §5 设 `margin_*`、`bar_ratio`、`font_scale`、`logo_scale`、`separator`、`show_labels`、`bar_position`、颜色。关键：旧 4 模板的 `default_for` 产出的 `effective_margins()` 必须等于旧 `border_ratio`（保证 layout 未改时行为不变、且 Task 3 改 layout 后 golden 不变）。

各模板 `default_for` 具体值（旧 4 个严格照旧，仅额外填 `margin_*`=border_ratio）：
- WhiteStudio：全部 = base（border .04、bar .06、font .45、logo .6、白底 #1a1a1a 前景）；margins=[.04;4]
- BlackBar：同 base 但 bg `[17,17,17,255]` fg `[240,240,240,255]`；margins=[.04;4]
- Minimal：border .015 bar .035 font .55 logo .5；margins=[.015;4]
- Magazine：border .08 bar .11 font .38 logo .5；margins=[.08;4]
- Polaroid：margins=[.03,.14,.03,.03] bar_ratio .035 font .45 logo .0（拉起用滑块）白底
- TwoColumn：margins=[.04;4] bar .06 font .45 logo .5 白底；`enabled`/默认字段开 Camera（GUI 侧处理）
- TopBar：margins=[.04,.06,.04,.04] bar .06 font .45 logo .6 `bar_position=Top` 白底
- CaptionCard：margins=[.03,.03,.06,.03] bar .06 font .5 logo .5 `show_labels=true` `label_ratio=.62` `bar_position=Left` 白底

`new` 模板 `bar_position`/`separator`/`show_labels`/`label_ratio` 取默认；`bar_position` 默认值 = 该 spec 的 `bar_edge`（在 `default_for` 里显式赋值）。

- [ ] **Step 5: layout.rs 加过渡臂（编译保命，零新行为）**

新枚举使旧 `layout.rs:126` 的穷尽 `match tpl` 编译失败。把该 match 的两个 arm 改为显式旧变体 + 一条兜底，新变体在 Task 3 前不被 GUI/e2e 使用：

```rust
match tpl {
    TemplateId::WhiteStudio | TemplateId::BlackBar | TemplateId::Minimal => { /* 原样 */ }
    TemplateId::Magazine => { /* 原样 */ }
    // Interim: new templates render like White Studio until the spec
    // engine lands (Task 3). Never reachable from GUI/e2e before then.
    _ => { /* 复制 WhiteStudio 分支体 */ }
}
```
（`two_lines = matches!(tpl, TemplateId::Magazine) && ...` 与 `line_h` 等其余逻辑不动。）

Run: `cargo check -p framely-core`
Expected: 通过。

- [ ] **Step 6: 运行确认通过 + 全量旧测试仍绿**

Run: `cargo test -p framely-core`
Expected: template.rs 新测试 PASS；golden.rs PASS；layout.rs / render.rs 等全部 PASS（layout 未改，旧 4 模板行为不变）。

- [ ] **Step 6: 提交**

```bash
git add core/src/template.rs core/tests/template.rs
git commit -m "feat: add TemplateSpec, new template ids, asymmetric margin + option fields with serde compat"
```

---

## Task 3: `layout()` 引擎重写（spec 驱动，golden 必须全绿）

高风险改动，单独成提交。签名不变：`layout(tpl, photo_w, photo_h, brand, logo_ratio, theme, fields) -> FrameLayout`。边距用 `theme.effective_margins()`；分隔符用 `theme.separator`；条位用 `theme.bar_position`（Left 类模板恒为 Left）。

**Files:**
- Modify: `core/src/layout.rs`

**Interfaces:**
- Consumes: `TemplateId::spec()`、`ThemeConfig::effective_margins()`、`ThemeConfig::separator_str`、`Field::Camera`
- Produces: 与现状同构的 `FrameLayout`/`DrawOp`（不新增 op）；4 个私有结构函数 `bar_single` / `bar_two_line` / `bar_two_column` / `bar_stacked_labeled`

- [ ] **Step 1: 先加失败测试（新引擎才有的行为）**

在 `core/tests/layout.rs` 追加（本任务先加与引擎直接相关的几条，Task 4 补其余）：

```rust
#[test]
fn bar_separator_bar_uses_pipe_and_keeps_text() {
    let mut theme = ThemeConfig::default_for(TemplateId::WhiteStudio);
    theme.separator = framely_core::template::Separator::Bar;
    let l = layout(TemplateId::WhiteStudio, 2000, 1333, Brand::Sony, 2.0, &theme, &all_fields());
    let joined = texts(&l).join(" ");
    assert!(joined.contains('|'), "pipe separator missing: {joined}");
    assert!(joined.contains("f/1.8") && joined.contains("ISO100"), "fields swallowed: {joined}");
}

#[test]
fn top_bar_sits_above_the_photo() {
    let l = lay(2000, 1333, TemplateId::TopBar, &all_fields(), Brand::Sony);
    // TopBar: photo y = mt + bar  >  mt (a bottom-bar photo starts at mt).
    let (px, py, _, _) = l.photo_rect;
    assert!(py > px, "top bar pushes the photo down: py {py} vs mt {px}");
    let bar_bottom = l.ops.iter().find_map(|o| match o {
        DrawOp::Text { rect, .. } => Some(rect.1 + rect.3),
        _ => None,
    }).expect("text op");
    assert!(bar_bottom <= py, "bar rect must stay above the photo");
}
```

> TopBar 的默认 `bar_position=Top`：`photo_rect.y = margin_top + bar`，且条内 Text rect 完全位于照片上方（rect.1+height ≤ photo_rect.1）。注意不能用 `l.border` 断言——TopBar 四边不对称（top .04 / bottom .06），`border` 字段只是四者最大值。

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p framely-core --test layout`
Expected: FAIL（现 layout 仍用 `match tpl` 只认 4 变体，TopBar 走不到 / 分隔符不变）。

- [ ] **Step 3: 重写 layout 为 spec 驱动**

用下述骨架整体替换 `layout.rs` 的 `layout()` 与 `join`（`Align`/`DrawOp`/`FrameLayout`/`Fitted`/`fit_fields`/`estimate_width`/`logo_y` 保留；`SEP` 常量删，`join` 改带分隔符参数）。核心：

```rust
pub fn layout(tpl, photo_w, photo_h, brand, logo_ratio, theme, fields) -> FrameLayout {
    let spec = tpl.spec();
    let short = photo_w.min(photo_h) as f64;
    let m = theme.effective_margins();
    let (mt, mb, ml, mr) = (m[0], m[1], m[2], m[3]);
    let mt = (short * mt).round().max(1.0) as u32;
    let mb = (short * mb).round().max(1.0) as u32;
    let ml = (short * ml).round().max(1.0) as u32;
    let mr = (short * mr).round().max(1.0) as u32;
    let bar = (short * theme.bar_ratio).round().max(12.0) as u32;

    let edge = match spec.bar_edge {
        BarEdge::Left => BarEdge::Left,               // Caption Card: locked
        _ => theme.bar_position,                       // Bottom/Top overridable
    };

    // Canvas & photo rect; the bar occupies its own band on `edge`.
    let (bar_h, canvas_w, canvas_h, photo_x, photo_y);
    match edge {
        BarEdge::Bottom => {
            canvas_w = photo_w + ml + mr;
            canvas_h = photo_h + mt + mb + bar;
            photo_x = ml; photo_y = mt; bar_h = bar;
        }
        BarEdge::Top => {
            canvas_w = photo_w + ml + mr;
            canvas_h = photo_h + mt + mb + bar;
            photo_x = ml; photo_y = mt + bar; bar_h = bar;
        }
        BarEdge::Left => {
            // Caption Card: text lives inside the left margin; no extra band.
            canvas_w = photo_w + ml + mr;
            canvas_h = photo_h + mt + mb;
            photo_x = ml; photo_y = mt; bar_h = ml;   // bar thickness = left margin
        }
    }

    // Shared bar inputs (logo slot, font, inner pad) — mirror current math.
    let font = (bar_h as f64 * theme.font_scale).round().max(9.0) as u32;
    let inner_pad = (bar_h as f64 * 0.25).round() as u32;
    let (logo_slot, logo_h) = logo_metrics(brand, logo_ratio, theme, bar_h);
    let sep = ThemeConfig::separator_str(theme.separator);

    let mut ops = Vec::new();
    match spec.structure {
        BarStructure::SingleLine => bar_single(&mut ops, edge, photo_x, photo_y, canvas_w, canvas_h, bar_h, mt, font, inner_pad, logo_slot, logo_h, brand, theme, fields, sep, photo_w),
        BarStructure::TwoLine    => bar_two_line(&mut ops, edge, photo_x, photo_y, canvas_w, canvas_h, bar_h, mt, font, inner_pad, logo_slot, logo_h, brand, theme, fields, sep, photo_w),
        BarStructure::TwoColumn  => bar_two_column(&mut ops, edge, photo_x, canvas_w, bar_h, mt, font, inner_pad, logo_slot, logo_h, brand, theme, fields, sep, photo_w),
        BarStructure::StackedLabeled => bar_stacked_labeled(&mut ops, bar_h, ml, mt, font, inner_pad, logo_slot, logo_h, brand, theme, fields, sep),
    }

    FrameLayout { canvas_width: canvas_w, canvas_height: canvas_h,
        border: mt.max(mb).max(ml).max(mr), bar_height: bar_h,
        photo_rect: (photo_x, photo_y, photo_w, photo_h),
        background: theme.background, foreground: theme.foreground, ops }
}
```

**逐像素一致性关键（golden）**：`bar_single` 对 Bottom/Top 条、`logo_slot>0`、`center_group` 的数学必须与当前 `WhiteStudio|BlackBar|Minimal` 分支逐行等价：`region_x = ml + inner_pad + logo_slot + gap`、`text_region_w = photo_w - (2*inner_pad + logo_slot + gap)`、fit 用 `estimate_width`。`bar_two_line` 对 Magazine 等价于当前 `two_lines` 分支。**旧 4 模板的 `border` 用四边同值** → `ml=mr=mt=mb=旧border`，`region_x` 从 `border+inner_pad` 变 `ml+inner_pad`（相等）。`border` 字段（FrameLayout.border）旧代码 = 单一 border；新代码取 max（四边同值时相等），供旧测试 `assert_eq!(l.border, 53)` 仍成立。

`bar_single` 中：logo x 非居中 = `ml + inner_pad`；Text rect 非居中 = `(region_x, bar_y, text_region_w, bar)`，`bar_y` = Bottom: `mt + photo_h`；Top: `mt`。居中/`logo_y` 逻辑照搬当前实现，仅 `bar_y` 随 edge 变。

`logo_metrics` = 抽出的 `logo_h`/`logo_slot` 计算（等价当前 lines 80-85）。

`bar_stacked_labeled`（Caption Card）：值区 x ∈ `[inner_pad, ml - inner_pad]`，宽 `avail = ml - 2*inner_pad`。逐字段：标签行 `Text{content: field.label(), size = value_size*label_ratio, color: fg_with_alpha(150), align: Left, rect}`，值行 `Text{content, size: value_size, color: fg, align: Left, rect}`。自上而下堆叠，`value_size = font`，行高按 `fit_fields` 预算；logo（若有）置于条顶端（`DrawOp::Logo` 在 y=mt 附近）。装不下时按 `FIELD_DROP_ORDER` 丢字段（复用 fit_fields 变体或本地循环），每字段占「标签+值」两行高度预算。`show_labels=false` 时只发值行。

`bar_two_column`：左栏机型串（`Field::Camera` 值，字号 = font），右栏其余参数（字号 `0.85*font`，用 sep 连接）。左栏宽 = `measure_text`? 否——用 `estimate_width(model, font)`，上限 `0.4 * (canvas_w - 2*inner_pad)`。机型空/无 Camera → **退化为 bar_single 的 SingleLine 输出**（仅 1 个 Text op）。logo 置于左栏起点。

- [ ] **Step 4: 运行 layout 测试 + golden**

Run: `cargo test -p framely-core --test layout --test golden`
Expected: golden 72 用例 PASS（逐像素不变），layout 旧测试 PASS，新增 2 测试 PASS。

- [ ] **Step 5: 全量 core 测试 + clippy + fmt**

Run: `cargo test -p framely-core && cargo clippy -p framely-core --all-targets -- -D warnings && cargo fmt --check`
Expected: 全绿（render.rs 测试用 400×300 WhiteStudio，仍走 SingleLine，不变）。

- [ ] **Step 6: 提交**

```bash
git add core/src/layout.rs core/tests/layout.rs
git commit -m "refactor: drive layout() from TemplateSpec with a single engine (golden-preserved)"
```

---

## Task 4: 新模板单元测试补全 + Camera 端到端

**Files:**
- Modify: `core/src/exif.rs`（`Field::Camera` 派生，见 Step 3）
- Modify: `core/tests/layout.rs`
- Modify: `core/tests/exif.rs`（Sony fixture 带出 Camera 断言）

**Interfaces:**
- Consumes: Task 3 引擎、Task 2 `Field::Camera`

> 注：Task 3 已含 `bar_separator_bar_uses_pipe_and_keeps_text` 与 `top_bar_sits_above_the_photo`，此处不重复。

- [ ] **Step 1: 追加新模板/标签/出界测试**

在 `core/tests/layout.rs` 追加（覆盖 Review Focus + spec §7；helper 见文末）：

```rust
#[test]
fn polaroid_bottom_margin_dominates() {
    let l = lay(2000, 1333, TemplateId::Polaroid, &all_fields(), Brand::Sony);
    // top margin = round(1333*.03)=40; bottom band = mb + bar.
    let top = l.photo_rect.1;
    let bottom_band = l.canvas_height - (top + 1333);
    assert!(bottom_band >= 3 * top, "polaroid foot {bottom_band} vs top {top}");
}

#[test]
fn two_column_splits_model_and_params() {
    let l = lay(2000, 1333, TemplateId::TwoColumn, &camera_fields(), Brand::Sony);
    let t = texts(&l);
    assert_eq!(t.len(), 2, "model line + params line, got {t:?}");
    assert!(t.iter().any(|s| s.contains("ILCE-7RM4")), "model column missing: {t:?}");
}

#[test]
fn two_column_collapses_to_single_line_when_no_camera() {
    let l = lay(2000, 1333, TemplateId::TwoColumn, &all_fields(), Brand::Sony);
    assert_eq!(texts(&l).len(), 1, "no camera → one joined line, got {:?}", texts(&l));
}

#[test]
fn caption_card_pair_per_field_inside_left_margin() {
    // Short values on a 1333-short canvas: strip_w=80, inner pad 8,
    // avail 64, value size 16, label size 10 — every group fits.
    let l = lay(2000, 1333, TemplateId::CaptionCard, &short_fields(), Brand::Leica);
    assert_eq!(texts(&l).len(), 2 * short_fields().parts.len(), "label+value per field");
    for o in &l.ops {
        if let DrawOp::Text { rect, .. } = o {
            assert!(rect.0 + rect.2 <= l.photo_rect.0 + 1, "caption text leaves left margin: {rect:?}");
        }
    }
}

#[test]
fn caption_card_drops_long_fields_instead_of_overflowing() {
    // The long lens string cannot fit the 64px strip → dropped by order;
    // the exposure trio survives; nothing leaves the canvas.
    let l = lay(2000, 1333, TemplateId::CaptionCard, &all_fields(), Brand::Leica);
    let t = texts(&l);
    assert!(!t.iter().any(|s| s.contains("35mm")), "long lens value must be dropped: {t:?}");
    assert!(t.iter().any(|s| s.contains("f/1.8")), "exposure trio survives: {t:?}");
    for o in &l.ops {
        if let DrawOp::Text { rect, .. } = o {
            assert!(rect.1 + rect.3 <= l.canvas_height, "caption text overflowed vertically: {rect:?}");
        }
    }
}

#[test]
fn show_labels_off_halves_caption_text_ops() {
    let mut theme = ThemeConfig::default_for(TemplateId::CaptionCard);
    theme.show_labels = false;
    let off = layout(TemplateId::CaptionCard, 2000, 1333, Brand::Leica, 2.0, &theme, &short_fields());
    assert_eq!(texts(&off).len(), short_fields().parts.len(), "values only when labels off");
}

#[test]
fn margins_scale_linearly_per_side() {
    // Polaroid margins .03/.14/.03/.03, bar .035, short sides 1000→2000:
    // every derived length doubles exactly (30→60, 140→280, 35→70).
    let s = lay(1500, 1000, TemplateId::Polaroid, &all_fields(), Brand::Sony);
    let b = lay(3000, 2000, TemplateId::Polaroid, &all_fields(), Brand::Sony);
    assert_eq!(b.photo_rect.1, s.photo_rect.1 * 2); // top margin
    let s_bottom = s.canvas_height - s.photo_rect.1 - 1000;
    let b_bottom = b.canvas_height - b.photo_rect.1 - 2000;
    assert_eq!(b_bottom, s_bottom * 2); // bottom margin + bar
}

#[test]
fn all_eight_templates_stay_inside_canvas() {
    for tpl in TemplateId::ALL {
        let l = lay(1600, 1067, tpl, &camera_fields(), Brand::Leica);
        for o in &l.ops {
            if let DrawOp::Text { rect, .. } = o {
                assert!(rect.0+rect.2 <= l.canvas_width && rect.1+rect.3 <= l.canvas_height,
                    "{tpl:?} text outside canvas {rect:?} vs {}x{}", l.canvas_width, l.canvas_height);
            }
        }
    }
}
```

补 helper（`core/tests/layout.rs` 顶部）：

```rust
fn short_fields() -> Fields {
    Fields {
        parts: vec![
            (Field::Aperture, "f/2.8".into()),
            (Field::Shutter, "1/250".into()),
            (Field::Iso, "ISO100".into()),
        ],
        custom_line: None,
    }
}

fn camera_fields() -> Fields {
    let mut f = all_fields();
    f.parts.insert(0, (Field::Camera, "ILCE-7RM4".into()));
    f
}
```

- [ ] **Step 2: 运行确认失败后在 `exif.rs` 派生 `Field::Camera`**

裁决：`Camera` 直接在 `from_bytes` 里进 `parts`（预览/导出/批量三处同构，无新方法）。默认模板外观不受影响：预览/导出经 `apply_enable` 过滤，默认 `enabled` 不含 Camera（裁决 5）；批量沿用既有「全字段不过滤」行为，结果自然多出机型。

`core/src/exif.rs` 的 `ORDER` 扩为 7 项（`Camera` 置首，机型在最前与 Two-Column 左栏语义一致）：

```rust
const ORDER: [Field; 7] = [
    Field::Camera,
    Field::Lens,
    Field::Focal,
    Field::Aperture,
    Field::Shutter,
    Field::Iso,
    Field::Date,
];
```

`from_bytes` 里 `model` 读出后（在 lens 之前）加派生：

```rust
// Body model as its own field: the camera that took the shot (the Lens
// field may show a lens name instead). Trim to keep the joined line clean.
let model_trim = model.trim().to_string();
if !model_trim.is_empty() {
    parts.push((Field::Camera, model_trim));
}
```

`core/tests/exif.rs` 追加：

```rust
#[test]
fn camera_field_carries_the_body_model() {
    let info = PhotoInfo::from_bytes(&fixture("sony.jpg"));
    assert_eq!(field_text(&info, Field::Camera).as_deref(), Some("ILCE-7RM4"));
    assert_eq!(info.fields.parts.first().map(|(f, _)| *f), Some(Field::Camera));
}

#[test]
fn no_model_means_no_camera_field() {
    let info = PhotoInfo::from_bytes(&fixture("plain.jpg"));
    assert!(info.fields.parts.is_empty());
}
```

Run: `cargo test -p framely-core`
Expected: 全 PASS。

- [ ] **Step 3: 提交**

```bash
git add core/tests/layout.rs core/tests/exif.rs core/src/exif.rs
git commit -m "test: cover new templates, labels, margins; add Field::Camera to PhotoInfo"
```

---

## Task 5: GUI — i18n 文案 + 模板分组 + 按模板适配控件集

**Files:**
- Modify: `app/src/i18n.rs`
- Modify: `app/src/panels.rs`
- Modify: `app/src/app.rs`（默认 `enabled` + 保存归一化）
- Test: `app/src/i18n.rs`（`#[cfg(test)]` 内断言 8 模板名 + Camera 标签）

**Interfaces:**
- Consumes: `TemplateId::ALL`(8)、`TemplateId::spec()`、`ThemeConfig` 新字段
- Produces: `theme_panel` 呈现新控件；`i18n::template_name`/`field_label` 覆盖 8 模板与 Camera

- [ ] **Step 1: i18n 中文 + 断言测试**

`template_name` 的 match 加：`Polaroid=>"拍立得"`、`TwoColumn=>"双栏相机"`、`TopBar=>"顶部参数条"`、`CaptionCard=>"侧边签条"`。`field_label` 加 `Camera=>"机身"`。文件末尾加：
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_templates_have_chinese_names() {
        for t in framely_core::template::TemplateId::ALL {
            assert_ne!(template_name(Lang::Zh, t), t.display_name(), "{t:?} missing zh name");
        }
    }
    #[test]
    fn camera_label_is_body() {
        assert_eq!(field_label(Lang::Zh, framely_core::template::Field::Camera), "机身");
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p framely-app`
Expected: FAIL（match 非穷尽 → 编译错，或断言失败）。

- [ ] **Step 3: i18n 实现（Step 1 的 match 追加即实现）**

补齐 match 臂，使编译通过。

- [ ] **Step 4: `theme_panel` 控件集 + 分组 + 可见性**

`app/src/panels.rs` 的 `theme_panel` 中，把顶部模板 radio 循环整体替换为分组（i18n 键内联，与现有 paired-string 模式一致；`CollapsingHeader::new(...).default_open(holding).show(ui, ...)` 即 egui 0.37 写法）：

```rust
ui.heading(lang.select("Template", "模板"));
ui.separator();
let bar_group = [
    TemplateId::WhiteStudio,
    TemplateId::BlackBar,
    TemplateId::Minimal,
    TemplateId::Magazine,
    TemplateId::TwoColumn,
    TemplateId::Polaroid,
];
let special_group = [TemplateId::TopBar, TemplateId::CaptionCard];
for (title_en, title_zh, list) in [
    ("Bottom bar", "底条类", &bar_group[..]),
    ("Special structures", "特殊结构", &special_group[..]),
] {
    let holding = list.contains(template);
    egui::CollapsingHeader::new(lang.select(title_en, title_zh))
        .default_open(holding)
        .show(ui, |ui| {
            for &t in list {
                if ui.radio_value(template, t, template_name(lang, t)).changed()
                    && t == TemplateId::TwoColumn
                    && !enabled.contains(&Field::Camera)
                {
                    enabled.push(Field::Camera); // spec §5: Camera 默认开（仅该模板）
                }
            }
        });
}

let cfg = theme_for(themes, *template);   // 现有行不变
let spec = template.spec();               // radio 可能改过 template，此处重取
```

`cfg`（当前主题的 `&mut ThemeConfig`）就绪后，控件集按可见性渲染。哨兵归一化（一次性，使滑块有真实起点）：

```rust
if cfg.margin_top < 0.0 {
    let b = cfg.border_ratio;
    cfg.margin_top = b;
    cfg.margin_bottom = b;
    cfg.margin_left = b;
    cfg.margin_right = b;
}
```

边宽 ×4（替换原单「Border」滑块），拖动即同步均值进 `border_ratio`：

```rust
for (en, zh, slot) in [
    ("Margin top", "边宽·顶", 0),
    ("Margin bottom", "边宽·底", 1),
    ("Margin left", "边宽·左", 2),
    ("Margin right", "边宽·右", 3),
] {
    let f = match slot {
        0 => &mut cfg.margin_top,
        1 => &mut cfg.margin_bottom,
        2 => &mut cfg.margin_left,
        _ => &mut cfg.margin_right,
    };
    slider(ui, lang.select(en, zh), f, 0.005..=0.25);
}
cfg.border_ratio =
    (cfg.margin_top + cfg.margin_bottom + cfg.margin_left + cfg.margin_right) / 4.0;
```

（Polaroid 的 0.14 底边距要求上限 ≥0.15，取 0.25。）

条位置分段按钮（Caption Card 隐藏）：

```rust
if spec.bar_edge != BarEdge::Left {
    ui.horizontal(|ui| {
        ui.label(lang.select("Bar position", "参数条位置"));
        for (edge, en, zh) in [
            (BarEdge::Bottom, "bottom", "下"),
            (BarEdge::Top, "top", "上"),
        ] {
            ui.selectable_value(&mut cfg.bar_position, edge, lang.select(en, zh));
        }
    });
}
```

条高 / 字号 / 字段间距 / 整体居中 / 分隔符（Caption Card 全部隐藏——裁决 4、6）：

```rust
if spec.structure != BarStructure::StackedLabeled {
    slider(ui, lang.select("Bar height", "参数条高度"), &mut cfg.bar_ratio, 0.02..=0.20);
    slider(ui, lang.select("Font size", "字号"), &mut cfg.font_scale, 0.2..=0.9);
    slider(ui, lang.select("Field spacing", "字段间距"), &mut cfg.part_spacing, 0.2..=1.5);
    ui.checkbox(&mut cfg.center_group, lang.select("Center logo + fields", "Logo 与参数整体居中"));
    egui::ComboBox::from_label(lang.select("Separator", "分隔符"))
        .selected_text(match cfg.separator {
            Separator::Dot => "·",
            Separator::Bar => "|",
            Separator::Space => lang.select("space", "空格"),
        })
        .show_ui(ui, |ui| {
            for (s, lab) in [
                (Separator::Dot, "·"),
                (Separator::Bar, "|"),
                (Separator::Space, lang.select("space", "空格")),
            ] {
                ui.selectable_value(&mut cfg.separator, s, lab);
            }
        });
}
```

字段标签（全部显示开关；比例仅堆叠结构显示；裁决 6）：

```rust
ui.checkbox(&mut cfg.show_labels, lang.select("Field labels", "字段标签"))
    .on_hover_text(lang.select(
        "only applies to Caption Card",
        "仅侧边签条生效",
    ));
if spec.structure == BarStructure::StackedLabeled && cfg.show_labels {
    slider(ui, lang.select("Label size", "标签比例"), &mut cfg.label_ratio, 0.3..=0.9);
}
```

Logo 区（现有「Logo size / Logo vertical」滑块与 Mono/Color radio 不变）。

字段区列表在 panels.rs 现有两处 `for f in [...]` 数组与 app.rs `FramelyApp::default().enabled` 之外**不**加 Camera——只在 `panels.rs::fields_panel` 与 `theme_panel` 的字段数组加 `Field::Camera`（首项），使开关列表出现「机身」；`enabled` 默认值不变（裁决 5）。

面板底部固定灰字（theme_panel 末尾）：

```rust
ui.add_space(8.0);
ui.label(RichText::new(lang.select(
    "Some options don't apply to the current template.",
    "部分选项不适用于当前模板。",
))
.weak().small());
```

- [ ] **Step 5: `app.rs` 保存归一化**

`FramelyApp::default().enabled` 不变（5 字段）。`save_preset()` 中构造 `PresetFile` 前：

```rust
let mut theme = self.theme();
theme.normalize_for_save();
```
并把 `theme` 放进 payload。`load_preset` 路径不变（载入的 margin_* 已具值，border_ratio 亦真）。

- [ ] **Step 6: 运行 app 测试 + 编译**

Run: `cargo test -p framely-app && cargo build -p framely-app`
Expected: i18n 测试 PASS；编译通过。

- [ ] **Step 7: 提交**

```bash
git add app/src/i18n.rs app/src/panels.rs app/src/app.rs
git commit -m "feat(gui): 8-template picker, unified per-template option controls, Camera field"
```

---

## Task 6: 文档 + 状态更新 + 全量门禁

**Files:**
- Modify: `README.md`、`README.zh-CN.md`
- Modify: `docs/superpowers/specs/2026-10-25-template-system-design.md`

- [ ] **Step 1: README 模板表（双语）更新为 8 个**

把「4 templates (White Studio / Black Bar / Minimal / Magazine)」改为 8 个列表；中文 README 同步，并新增一句说明新增可调项（四边独立边宽、条位、分隔符、字段标签、机身）。

- [ ] **Step 2: 设计稿状态改「已实施」**

把 spec 首块 `- 状态：已与用户逐段确认，待实施` 改为 `- 状态：已实施（v0.3.0）`。

- [ ] **Step 3: 全量门禁**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check`
Expected: 全绿。

- [ ] **Step 4: 提交**

```bash
git add README.md README.zh-CN.md docs/superpowers/specs/2026-10-25-template-system-design.md
git commit -m "docs: update template list to 8 and mark template-system spec implemented"
```

---

## 验证（端到端）

1. `cargo test --workspace` 全绿，其中 `golden_matches_baseline` 证明 4 旧模板逐像素不变。
2. `cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --check` 无输出。
3. GUI 人工验证（现有约定）：依次选 8 个模板，确认 Caption Card 隐藏条高/分隔符、Top Bar 参数在上、Polaroid 底边加宽、Two-Column 机型分栏、侧边签条标签在上；旧预设 JSON 载入外观不变；保存新预设后用文本编辑器确认含 4 个 `margin_*` 与均值 `border_ratio`。
4. `cargo run -p framely-app --example e2e` 跑通（可选，人工目检 /tmp/framely-e2e）。
