# Framely 模板体系设计（Template System）

- 日期：2026-10-25
- 状态：已与用户逐段确认，待实施
- 前置：本文取代 `2026-10-24-photo-frame-design.md` 中"template.rs + layout.rs — 主题与布局"一节；其余部分（EXIF 策略、渲染管线、批量导出等）仍然有效
- 基线提交：`4648448`（v0.2.0）

## 1. 目标

来自对主流 EXIF 边框工具（Framio、Chexif、Exif Styler、MetaMark、ExifFrame、
exif-frame.com）的布局对标分析：本类工具的布局能力是 5 个正交维度——参数区
位置、四边不对称边宽、条内排版结构、文字层级、品牌标识位置——而 Framely 当前
只覆盖"底条 + 等宽边距 + 单/双行 + 无标签"这一窄点。

本设计把模板从"match 分支"改为"数据 spec + 单一布局引擎"，在不改变现有 4 个
模板外观的前提下扩充到 8 个常用模板，并把可调项统一为一套按模板适配的控件集。

用户已确认的决策：

| 项 | 决策 |
|---|---|
| 实现路径 | 方案 A：`TemplateSpec` 数据 + 通用 `layout()` 引擎（否决 B 分支堆叠 / C 通用盒模型引擎） |
| 现有模板 | 保留 White Studio / Black Bar / Minimal / Magazine 的位置与默认值不变 |
| 新增模板 | Polaroid / Two-Column / Top Bar / Caption Card（4 个全做） |
| 可调项粒度 | 统一大集、按模板适配（面板隐藏不适用项），不做每模板独立配置结构 |
| "竖排"语义 | 横排小字逐行堆叠（美术馆标签实际做法），不做 glyph 旋转 |

## 2. 非目标

- 照片内浮层水印 / overlay（遮挡画面，与边框定位相悖）
- 目标宽高比裁切（属图片编辑，v1 已明确范围外）
- 任意拖拽可视化布局编辑器（与"模板 + 参数"模型和即时模式 GUI 冲突）
- 安装包 / 签名 / 公证
- 每模板独立可调项结构（预设格式分叉）

## 3. 架构

### 3.1 模板 = 数据

`core/src/template.rs` 新增两个枚举与一个结构：

```rust
/// Where the parameter bar sits relative to the photo.
pub enum BarEdge { Bottom, Top, Left }

/// How the bar's contents are arranged.
pub enum BarStructure {
    /// logo + one joined line (right-anchored, or centered as a group)
    SingleLine,
    /// existing Magazine behaviour: two right-anchored lines, one logo
    TwoLine,
    /// camera model (Medium weight) left, parameters right
    TwoColumn,
    /// narrow side bar: one small group per field, label above value
    StackedLabeled,
}

pub enum Separator { Dot, Bar, Space }

/// Everything that distinguishes one template's shape from another.
/// A template is one of these plus a default ThemeConfig.
pub struct TemplateSpec {
    pub id: TemplateId,
    pub display_name: &'static str,
    pub bar_edge: BarEdge,
    /// (top, bottom, left, right) as a fraction of the photo's short side
    pub margins: [f64; 4],
    pub structure: BarStructure,
    pub separator: Separator,
    /// Label size as a fraction of the value size (StackedLabeled).
    pub label_ratio: f64,
}

impl TemplateId {
    /// The template's shape. The switch lives here so the list of templates
    /// and their geometry cannot drift apart.
    pub fn spec(self) -> &'static TemplateSpec;
}
```

`TemplateId` 扩为 8 个变体，`ALL`、`id()`（稳定 kebab-case：`polaroid`、
`two-column`、`top-bar`、`caption-card`）与 `display_name()` 随之扩展。
`TemplateId` 的 serde 表示不变，因此旧预设里的 `"black-bar"` 等字符串照旧解析。

### 3.2 布局 = 一个引擎

`core/src/layout.rs::layout()` 重写为按 spec 驱动的单一函数，取代现在的
`match tpl` 三个分支：

```
1. 边距   margin_i = round(short * theme.margins[i]).max(1)
2. 条厚   bar      = round(short * theme.bar_ratio).max(12)
3. 画布   照片 + 四边边距 + 条厚（条所在那条边额外加条厚）
4. 条内   按 spec.structure 分派到 4 个小函数，各自返回 Vec<DrawOp>
```

四个条内结构函数共享同一批输入（logo 槽宽、可用宽度、fitted 字段、分隔符宽度、
标签可见性），互不知晓对方存在。`fit_fields`（缩字号 → 按 `FIELD_DROP_ORDER`
丢字段）逻辑不变，只是"可用宽度"按结构计算：

- `SingleLine` / `TwoLine` / `TwoColumn`：条宽减去左右内边距；`TwoColumn`
  进一步把条内空间分给左右两栏（左栏宽 = 机型串实测宽，上限 40% 条宽）。
- `StackedLabeled`：可用宽 = 左边距条宽 − 2×内边距；按字段数预算行高，
  装不下时从 `fit_fields` 的丢弃序列取字段。

logo 摆放规则收敛为一处："条内头部槽位"（Bottom/Top 条 = 左端，Left 条 = 顶端），
`center_group` 与 `logo_y_offset` 作为对该槽位的修饰，与 v0.2.0 语义一致。

`FrameLayout` 增加 `foreground`（已在 v0.2.0 落地，供无文字时的单色 logo 取色）。
`DrawOp` 不新增变体——堆叠与双栏都拆成既有 `Text` + `Logo` op，
`render::compose` 因此完全不改（仅 `Text` 的 `rect`/`align`/`size_px` 由新引擎填）。

### 3.3 数据流

与现状同构，仅"模板形状"的来源从代码分支变为数据：

```
选图 → exif → Brand + Fields(+Camera)
     → theme(TemplateId) + TemplateId::spec()
     → layout(): 边距 / 条厚 / 条内结构 → Vec<DrawOp>
     → render::compose（不变）→ jpeg 导出（不变）
```

## 4. ThemeConfig 扩展

新字段一律 `#[serde(default)]`，旧预设文件无需迁移即可载入。

| 字段 | 类型 | 语义 | 取代 / 新增 |
|---|---|---|---|
| `margin_top` | f64 | 顶边距 / 短边 | 取代 `border_ratio` |
| `margin_bottom` | f64 | 底边距 | 同上 |
| `margin_left` | f64 | 左边距 | 同上 |
| `margin_right` | f64 | 右边距 | 同上 |
| `bar_position` | `BarEdge` | 用户覆盖模板默认条位（仅 Bottom/Top 可覆盖） | 新增 |
| `separator` | `Separator` | 点 ` · ` / 竖线 ` \| ` / 空格 | 新增 |
| `show_labels` | bool | 字段上方小标签 | 新增 |
| `label_ratio` | f64 | 标签字号 / 值字号（默认取 spec） | 新增 |
| `border_ratio` | f64 | **保留**，兼容字段 | 见 4.1 |
| 其余（`bar_ratio`、`font_scale`、`logo_scale`、`logo_y_offset`、`center_group`、颜色、`part_spacing`） | | 不变 | |

### 4.1 兼容规则（只有一处）

载入时：若 `margin_*` 缺省而 `border_ratio` 存在 → 四边 := `border_ratio`。
保存时：全量写出四个 `margin_*`，并写 `border_ratio` = 四者均值，使旧版程序
读到新预设不报错（只是丢掉不对称）。四边一致时两组字段等价。

### 4.2 新增 `Field::Camera`

`Field` 枚举加 `Camera`（值来自 `PhotoInfo.model`，已在 exif.rs 提取）。
`Field::Camera` 参与开关与顺序，`FIELD_DROP_ORDER` 中排在 `Date` 之后。
`i18n::field_label` 中文 = "机身"。

### 4.3 脏检查

`ThemeConfig::hash` 手写实现纳入全部新字段（f64 写 bits、bool/枚举写判别值），
保证面板任一改动都触发预览重渲染。

## 5. 8 个模板的默认值

现有 4 个的外观**必须逐像素不变**（见 §7 golden 测试）。

| 模板 | 条位 | 边距 %（top/bottom/left/right） | 结构 | 分隔符 | 标签 | logo | 备注 |
|---|---|---|---|---|---|---|---|
| White Studio | 下 | 4/4/4/4 | SingleLine | Dot | 关 | 条内左，scale 0.6 | 现状不变 |
| Black Bar | 下 | 4/4/4/4 | SingleLine | Dot | 关 | 条内左，0.6，单色白 | 现状不变 |
| Minimal | 下 | 1.5×4 | SingleLine | Dot | 关 | 条内左，0.5 | 现状不变 |
| Magazine | 下 | 8/8/8/8 | TwoLine | Dot | 关 | 条内左，0.5 | 现状不变 |
| **Polaroid** | 下 | 3/**14**/3/3 | SingleLine | Dot | 关 | scale 0（可拉起） | 下边加宽是灵魂；`bar_ratio` 0.035 |
| **Two-Column** | 下 | 4/4/4/4 | TwoColumn | Dot | 关 | 左栏起点，0.5 | 机型空时退化单行；`Field::Camera` 默认开 |
| **Top Bar** | **上** | 4/6/4/4 | SingleLine | Dot | 关 | 条内左，0.6 | 参数在照片上方 |
| **Caption Card** | **左** | 3/3/**6**/3 | StackedLabeled | 不适用 | **开** | 条顶端，0.5 | `bar_position` 锁定为 Left |

Polaroid 默认不显示 logo 是**默认值**而非能力限制——`logo_scale` 滑块仍可拉起。

## 6. GUI

### 6.1 模板列表

主题面板顶部 8 个 radio，分两组 `CollapsingHeader`（"底条类" = White Studio /
Black Bar / Minimal / Magazine / Two-Column / Polaroid；"特殊结构" = Top Bar /
Caption Card），避免长列表。`i18n::template_name` 加中文：拍立得 / 双栏相机 /
顶部参数条 / 侧边签条。

### 6.2 控件集（按模板隐藏不适用项）

| 控件 | 适用 |
|---|---|
| 边宽 ×4（顶/下/左/右，替换原单滑块；改动同步写 `border_ratio` = 均值） | 全部 |
| 条位置分段按钮（上/下） | 除 Caption Card |
| 条高 / 字号 / 字段间距 / 整体居中 / 分隔符下拉 | 除 Caption Card（分隔符） |
| Logo 大小 / 上下位置 / 单色或彩色 | 全部（Caption Card logo 位于条顶端） |
| 字段标签开关 + 标签比例 | 全部 |
| 机身字段开关 | 全部（Two-Column 默认开） |

面板底部固定一行灰字说明"部分选项不适用于当前模板"，避免用户找不到控件。

### 6.3 堆叠标签的取色

标签色 = `foreground` 的 alpha 改为 150，不新增颜色控件。`show_labels` 关闭后
StackedLabeled 退化为纯值逐行。

## 7. 测试

**Golden 回归（核心护栏）**：实施前先在 `4648448` 上为 4 个旧模板生成基准
——3 组画幅（2000×1333 / 1333×2000 / 1600×1600）× 3 品牌（Sony / Leica /
Unknown）× 单色/彩色 logo，用 `render::compose` 出 PNG 并存 sha256 到新测试的
fixtures。新引擎必须产出**逐像素一致**的结果；任何差异按 bug 处理。

单元测试（`core/tests/layout.rs` 等）：

- Polaroid：`margin_bottom >= 3 * margin_top`，且画布高 = 照片 + 边距 + 条
- Two-Column：两个 Text op（机型栏 + 参数栏），机型栏 Medium、右栏参数；机型
  为空时退化为 SingleLine 形状（仅 1 个 Text op）
- Top Bar：条 rect 的 y < 照片 rect 的 y
- Caption Card：每字段 2 个 Text op（标签 + 值），全部落在左边距 rect 内不出界；
  字段过多时按 drop order 丢字段而非出界
- 分隔符三种各按 `measure_text` 实测宽度参与排版（竖线不吞字）
- `show_labels` 开关增减 Text op 数
- `Field::Camera` 进开关列表与 drop order
- serde：旧格式（仅 `border_ratio`）载入 → 四边同值；新格式 roundtrip 无损；
  `TemplateId` 新变体的 kebab-case id 断言
- 四边 margin 各自随画幅 ×2 线性缩放
- 无模板出界（现有测试扩展到 8 个模板）

既有全套（format / brand / exif / jpeg / render / template / batch / i18n）
保持绿；`cargo clippy --workspace --all-targets -- -D warnings`、
`cargo fmt --check`。GUI 交互按现有约定人工验证。

## 8. 错误处理

无新增失败模式：全部是数据驱动的几何计算，输入非法（负边距、极端比例）由
`.max(1.0)` / `.clamp()` 吸收。Caption Card 窄条装不下堆叠走既有丢字段路径。
不引入新错误变体。

## 9. 实施顺序建议

1. Golden fixtures 生成脚本 + 基准入库（先锁死旧行为）
2. `TemplateSpec` / 枚举 / `ThemeConfig` 扩展 + serde 兼容（旧模板仍走旧代码）
3. `layout()` 引擎重写，跑 golden 至全绿
4. 新模板 spec + 默认值，单元测试
5. GUI 面板控件集与按模板可见性
6. 文档：README 双语模板表、`docs/superpowers/specs/` 状态更新

第 3 步是唯一高风险改动，护栏是 golden；它必须单独成提交，不与功能改动混合。

## 10. 兼容性影响一览

| 面 | 影响 |
|---|---|
| 旧主题预设 JSON | 可载入，外观不变（`border_ratio` 展开为四边） |
| 旧版程序读新预设 | 不崩，丢掉不对称与条位（`border_ratio` 均值兜底） |
| `ThemeConfig` 公开 API | 加字段，不改既有字段语义 |
| `TemplateId` | 加变体；`ALL` 变 8；serde 表示不变 |
| `layout()` 签名 | 不变（新增行为全部经 `theme` + `TemplateId::spec()` 传入） |
| `render::compose` / `DrawOp` / jpeg 导出 | 不改 |
| 输出 EXIF 策略 | 不改（APP1 原样搬移 + orientation 改 1） |
| 已发版本 | v0.1.0 / v0.2.0 不动；本设计产出 v0.3.0 |
