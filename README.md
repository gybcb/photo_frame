# Framely

给照片加相机品牌边框的 macOS 桌面程序（Rust / eframe + egui）。

自动读取 EXIF，识别相机品牌并显示对应矢量 logo，填充镜头 / 焦距 / 光圈 /
快门 / ISO；缺失的字段可在界面手动补录。同一张图可单张实时预览，也可批量
导出。

## 构建与运行

```sh
cargo run -p framely-app          # 运行 GUI
cargo test --workspace            # 全部测试（core 逻辑 + batch 导出）
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

工具链：Rust 1.98+（edition 2024）。代码本身跨平台，但只在 macOS 上验证。

## 用法概览

- **选图**：拖拽进窗口，或工具栏按钮（`rfd` 文件对话框），支持多选进批量列表。
- **左侧主题面板**：4 个模板（White Studio / Black Bar / Minimal / Magazine），
  边宽、参数条高、字号、前/背景色、字段开关与顺序、logo 单色/官方彩色均可调。
  可调项可保存/载入为 JSON 预设（对话框默认目录为主目录）。
- **右侧字段面板**：每字段显示 EXIF 值，勾选"手动"后改为自定义文本；
  另有一条"整行自定义文本"（应对 Alpa 等机身不写镜头名的情况）。
  手动值只影响渲染，不改写导出文件的 EXIF。
- **底部批量导出**：选目录 → 并行逐张跑同一条渲染管线 → 进度条 + 逐张错误列表。
  单张失败（如损坏文件）只记录原因，不中断批次。
- **输出命名**：`<原名>_framed.jpg`，重名自动加 `_2`、`_3`……

预览与导出走同一条 `render::compose` 管线，只是画布缩放不同——这是
"所见即所得"的保证。

## 支持的品牌

EXIF `Make` 前缀匹配：Sony、Nikon、Canon、Leica、Hasselblad、Phase One、
Alpa。识别不出（`Unknown`）不是错误：走无 logo 通用样式，只显示参数文字。

Logo 为矢量 SVG，在目标像素尺寸直接栅格化（不是放大位图），因此任意尺寸
输出同样锐利。每个品牌两份变体：`.color.svg`（官方原色）与 `.mono.svg`
（渲染时注入主题前景色）。

> Black Bar 深色底若把 logo 切到"官方彩色"，多数品牌为黑色字标，会看不清——
> 该主题默认单色（白），属预期行为。

## EXIF 策略（重要）

导出 JPEG 时，原文件的 Exif APP1 段**原样搬移**（一个字节不重写），插在
新编码 JPEG 的 SOI 之后。不重写的理由：各 Rust EXIF 写入库公认的重写路径
会破坏 MakerNote 内部绝对偏移（kamadak-exif 作者亦明确说明该风险）。

两个有意的例外/代价：

1. **Orientation 例外**：解码时像素已按 orientation 转正，若原样携带
   （如 6），查看器会把成品再转一次。因此搬移前把 0x0112 标签的内联
   SHORT 值改写为 1——**仅此 2 字节**，MakerNote、所有偏移、其余标签
   不动；原本就没有或本来就是 1 的文件逐字节不变。
2. **PixelXDimension / PixelYDimension 仍为原图尺寸**，与加框后的更大画布
   不符。主流查看器不受影响，`exiftool -validate` 可能提示。

无 EXIF 的输入正常导出纯 JPEG，不报错。

Logo 资产来源与许可（Wikimedia Commons，PD-textlogo + Trademarked 提示；
净化步骤）见 `assets/brand/LICENSES.md`。字体 Inter（OFL）已内置进二进制，
输出不依赖系统字体。

## 项目结构

```
core/   framely-core — 无 GUI 依赖的全部逻辑
  exif.rs      EXIF 提取 → 展示字段
  brand.rs     Make/Model → 品牌匹配
  format.rs    快门/光圈/ISO/焦距/镜头名格式化（纯函数，全单测）
  template.rs  模板与可调主题（serde，预设即 JSON）
  layout.rs    比例化布局 → Vec<DrawOp>
  render.rs    SVG 栅格化 + 字形 + 合成（预览/导出共用）
  jpeg.rs      解码（含 orientation 转正）/ 带原 APP1 的导出
  assets.rs    内置 logo 与字体
app/    framely-app — eframe GUI 壳（面板、预览、批量）
```

## 范围外（v1）

RAW 解码、输出 PNG/TIFF、外部模板描述文件格式、Windows/Linux 打包验证、
图片编辑类附加功能（裁剪/调色）。

## 开发工具

`cargo run -p framely-app --example e2e` 用真实管线把测试图片导出到
`/tmp/framely-e2e/`，供目测与 EXIF 比对（需自装 `exiftool`，或用
Python Pillow；临时调试驱动，非产品功能）。
