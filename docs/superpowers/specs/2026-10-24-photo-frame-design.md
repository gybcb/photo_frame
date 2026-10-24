# photo_frame 设计文档

日期：2026-10-24
状态：已与用户逐段确认

## 目标

Rust 编写的 macOS 桌面 GUI 程序：给照片加相机品牌 logo 与拍摄参数边框。

- 自动读取 EXIF，识别相机品牌/型号/镜头/光圈/快门/ISO/焦距。
- 按机身品牌渲染对应 logo；logo 为矢量 SVG，按目标尺寸重新栅格化，任意分辨率锐利。
- EXIF 缺失字段可手动补录/覆盖（仅影响渲染，不改写导出 EXIF）。
- 品牌：索尼、尼康、佳能、徕卡、哈苏、飞思、Alpa。
- 工作流：单张实时预览 + 批量导出。

## 已确认决策

| 项 | 决策 |
|---|---|
| 形态 | GUI 桌面应用，`eframe` + `egui` |
| 输入 | 仅 JPEG/TIFF/PNG 等已导出图，不做 RAW |
| 输出 | JPEG，原 EXIF 段（APP1）原始字节整体搬移，不解析重写 |
| 边框 | 4 个内置主题可选，参数比例可调 |
| logo | Wikimedia Commons 官方 SVG 净化为矢量源，内置；Alpa 按官方形象手绘 |
| logo 颜色 | 单色（随主题前景色）/ 官方彩色 可切换，主题有默认倾向 |
| 处理管线 | 纯 CPU：image + resvg + ab_glyph + tiny-skia |

## 架构

Cargo workspace：

- `framely-core`：无 GUI 依赖的全部逻辑——exif（提取）、brand（识别）、format（字段格式化）、template/layout（模板与比例化布局）、render（栅格化合成语）、jpeg（解码/带 EXIF 导出）、assets（include_bytes! 内置 logo）、error。
- `framely-app`：eframe GUI 壳 + rfd 对话框 + rayon 批量导出。

**预览与导出共用同一条 core 管线**（仅画布缩放不同），保证所见即所得。

数据流：

```
选图 → exif::extract → PhotoMetadata
     → brand::resolve(make, model) → Brand
     → overrides 合并（手动值优先）
     → template::layout(canvas, fields, theme) → FrameLayout(DrawOps)
     → render::compose → RgbaImage
     → jpeg::save(编码 + 原APP1段插回SOI后) → <名>_framed.jpg
```

## 组件

### brand.rs

`Brand { Sony, Nikon, Canon, Leica, Hasselblad, PhaseOne, Alpa, Unknown }`。
EXIF Make 大写前缀匹配；Hasselblad 以型号 `V`/`CFV` 兜底；PhaseOne 兼容 `MAMIYA`、`LEICA CAMERA GEOSYSTEMS`。Unknown 走无 logo 通用模板，不报错。

### assets

`assets/brand/{brand}.color.svg` + `{brand}.mono.svg`（纯 path、规范 viewBox、无外部字体）。mono 渲染时注入主题前景色。`LICENSES.md` 记录来源与许可。缺资产回落 unknown 并 warn。

### format.rs

快门 `<1s → 1/250`、`≥1s → 2"`；光圈 `f/2.8`（整数不带小数）；ISO `ISO100`；焦距 `85mm`/`24-70mm`；镜头名清洗；日期 `YYYY-MM-DD`。

### template.rs / layout.rs

`FrameTemplate::layout(req) -> FrameLayout`。主题：`white-studio`（白边+底部左logo右参数条）、`black-bar`、`minimal`、`magazine`（宽边、logo 独占行、参数分行）。
所有几何按短边比例计算，任意尺寸视觉一致。可调项为 `ThemeConfig`（serde，JSON 预设存取）。

### jpeg.rs

解码按 EXIF orientation 旋转。导出：从原文件字节切出原始 APP1/Exif 段，插到编码结果的 SOI 之后。原图无 EXIF 则输出纯 JPEG。已知代价：EXIF 尺寸标签与实际像素不符（用户确认接受）。

### GUI

拖拽/选图（多选进批量列表）；中央预览（长边≤2048，脏标记重渲染，后台线程）；左主题面板；右字段面板（EXIF 值 + 手动覆盖 + 整行自定义文本）；底部批量导出（rfd 选目录、rayon、进度条、逐张错误）。导出文件名 `<原名>_framed.jpg`，重名加序号。

## 错误处理

`FrameError { UnsupportedFormat, Decode, Io, Encode }`；无 EXIF/未知品牌为降级路径而非错误。单张失败显示原因继续批次；预览失败显示错误文本。

## 测试

core 单测：format 边界、brand 全品牌+兜底、layout 比例线性/不溢出、render 栅格尺寸与 alpha、jpeg EXIF 搬移回归（合成含已知 EXIF 的 JPEG → 全管线 → 读回比对）与无 EXIF 降级。GUI 手动验证。

端到端：`cargo test/clippy/fmt` 全绿；exiftool 比对导出元数据；不同尺寸目测 logo 锐利；批量混入坏文件验证容错。

## v1 范围外

RAW、PNG/TIFF 无损输出、外部模板 DSL、跨平台打包验证、裁剪调色。
