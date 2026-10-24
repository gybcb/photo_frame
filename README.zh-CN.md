# Framely

[English](README.md) | [简体中文](README.zh-CN.md)

![CI](https://github.com/gybcb/photo_frame/actions/workflows/ci.yml/badge.svg)

给照片加相机品牌边框的桌面程序（Rust / eframe + egui）。自动读取 EXIF，
识别相机品牌并显示对应矢量 logo，填充镜头 / 焦距 / 光圈 / 快门 / ISO；
缺失字段可在界面手动补录。单张实时预览，多张并行导出。

## 构建与运行

```sh
cargo run --release -p framely-app  # 运行 GUI（日常用这个）
cargo test --workspace              # 全部测试（core 逻辑 + batch 导出）
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

工具链：Rust 1.98+（edition 2024）。代码面向 macOS / Windows / Linux 三
平台——GUI、文件对话框、字体探测、配置路径均按平台适配，且各有单测；CI
在三个平台上都构建与测试。交互使用目前只在 macOS 上人工验证过。每次发版
在 GitHub Release 页附三平台裸二进制（macOS/Linux 为 `.tar.gz`，Windows
为 `.zip`）——不做安装包（dmg/msi/AppImage），二进制未签名。Linux 包需要
系统装有 GTK3 与 X11/Wayland 运行库（桌面发行版一般自带）。

> 务必用 `--release` 运行。合成管线是逐像素 CPU 工作，debug 构建慢一个
> 数量级（3000px 单帧约 145ms vs 1.4ms），预览会明显卡顿；debug 仅供开发
> 调试。

### 在 macOS 上启动（未签名）

程序没有代码签名、也没有公证，启动方式有讲究：

- **自己构建（推荐）**：`cargo run --release`，或直接运行
  `target/release/framely-app`——自建的产物没有隔离属性（quarantine），
  终端里直接启动，不会被拦。
- **浏览器 / 聊天软件下载安装包**：macOS 会给文件打上隔离属性，Gatekeeper
  首次启动会拦截（提示"无法打开"或"已损坏"）。清除一次即可：

  ```sh
  xattr -d com.apple.quarantine /path/to/framely-app
  ```

  或右键文件 → **打开** → 在确认框再点 **打开**（被拦过一次后，系统设置
  → 隐私与安全性 里也会出现"仍要打开"按钮）。
- 用 `curl`/`wget` 下载的文件不会被打隔离属性——只有浏览器类下载才会。

## 用法概览

- **选图**：拖拽进窗口，或工具栏按钮（`rfd` 文件对话框），支持多选进批量
  列表。
- **左侧主题面板**：8 个模板（White Studio / Black Bar / Minimal /
  Magazine / 拍立得 Polaroid / 双栏相机 Two-Column / 顶部参数条 Top
  Bar / 侧边签条 Caption Card），按模板显示其适用可调项：四边独立边宽、
  参数条高、条位（上/下）、分隔符、字段标签、机身字段、字号、前/背景色、
  字段开关与顺序、logo 单色/官方彩色、logo 与参数整体居中均可调；可调项
  可保存/载入为 JSON 预设（对话框默认目录为主目录）。
- **右侧字段面板**：每字段显示 EXIF 值，勾选"手动"后改为自定义文本；另有
  一条"整行自定义文本"（应对 Alpa 等机身不写镜头名的情况）。手动值只影响
  渲染，不改写导出文件的 EXIF。
- **底部批量导出**：选目录 → 并行逐张跑同一条渲染管线 → 进度条 + 逐张
  错误列表。单张失败（如损坏文件）只记录原因，不中断批次。
- **输出命名**：`<原名>_framed.jpg`，重名自动加 `_2`、`_3`……

预览与导出走同一条 `render::compose` 管线，只是画布缩放不同——这就是
"所见即所得"的保证。

## 支持的品牌

EXIF `Make` 前缀匹配：Sony、Nikon、Canon、Leica、Hasselblad、Phase One、
Alpa。识别不出（`Unknown`）不是错误：走无 logo 通用样式，只显示参数文字。

Logo 为矢量 SVG，在目标像素尺寸直接栅格化（不是放大位图），因此任意尺寸
输出同样锐利。每个品牌两份变体：`.color.svg`（官方原色）与 `.mono.svg`
（渲染时注入主题前景色）。

> Black Bar 深色底若把 logo 切到"官方彩色"，多数品牌为黑色字标，会看不清
> ——该主题默认单色（白），属预期行为。

## 大图（拼接全景）

多张拼接的全景图可达数百兆像素。常驻内存的预览照片固定降到长边 2048 以
内再渲染（边框按比例、logo 矢量、文字运行时栅格化，小尺寸画布与全尺寸
视觉一致且文字更锐利）；导出时按需以原始分辨率重新解码，并在编码前释放
照片缓冲。批量导出限制 2 并发。约 286MP 的飞思两图拼接实测：打开+预览峰
值 ~2.3GB，导出瞬时峰值 ~3.8GB（此前直接报 "Memory limit exceeded"，或
顶到 5.4GB）。

## EXIF 策略（重要）

导出 JPEG 时，原文件的 Exif APP1 段**原样搬移**（一个字节不重写），插在新
编码 JPEG 的 SOI 之后。不重写的理由：各 Rust EXIF 写入库公认的重写路径会
破坏 MakerNote 内部绝对偏移（kamadak-exif 作者亦明确说明该风险）。两个有
意的例外/代价：

1. **Orientation 例外**：解码时像素已按 orientation 转正，若原样携带
   （如 6），查看器会把成品再转一次。因此搬移前把 `0x0112` 标签的内联
   SHORT 值改写为 1——**仅此 2 字节**，MakerNote、所有偏移、其余标签不
   动；原本就没有该标签或本来就是 1 的文件逐字节不变。
2. **PixelXDimension / PixelYDimension 仍为原图尺寸**，与加框后的更大画布
   不符。主流查看器不受影响，`exiftool -validate` 可能提示。

无 EXIF 的输入正常导出纯 JPEG，不报错。

Logo 资产来源与许可（Wikimedia Commons，PD-textlogo + Trademarked 提示；
净化步骤）见 `assets/brand/LICENSES.md`。字体 Inter（OFL）已内置进二进
制，输出不依赖系统字体。

## 语言

- 界面支持英文 / 中文，顶栏按钮一键切换，选择按平台持久化：macOS
  `~/.framely/config.json`（历史位置，保持不变以免升级丢配置）、Linux
  `$XDG_CONFIG_HOME/framely/config.json`（无则 `~/.config/framely/`）、
  Windows `%APPDATA%\framely\config.json`。
- 首次运行跟随系统 `LANG` 环境变量（`zh*` 前缀进中文）。Windows 没有
  `LANG`，默认英文，可随时切换。
- 中文字体不打包进二进制：运行时探测系统 CJK 字体——macOS PingFang / 冬青
  黑体 / 华文黑体；Windows 微软雅黑（`%WINDIR%\Fonts\msyh.ttc`）→ 黑体 →
  宋体；Linux Noto Sans CJK（Debian/Arch/Fedora 常见目录）→ 文泉驿微米黑
  / 正黑。探不到时界面保持纯英文、切换按钮不出现。
- CJK 字体追加在字体族末尾并缩放至 0.84：汉字填满 em 方框而拉丁只到 cap
  height，同字号下汉字会显大；冬青一类字体 `hhea.lineGap = 0.5em` 的行距
  还会让 egui 的逐字垂直居中把汉字顶出行框被裁。缩放同时解决显大与裁字
  （拉丁仍用默认字体、默认度量）。
- 边框内的文字（模板名、参数、Logo）恒为英文，与界面语言无关。

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

RAW 解码、输出 PNG/TIFF、外部模板描述文件格式、安装包/打包、图片编辑类
附加功能（裁剪/调色）。

## 开发工具

`cargo run -p framely-app --example e2e` 用真实管线把测试图片导出到
`/tmp/framely-e2e/`，供目测与 EXIF 比对（需自装 `exiftool`，或用 Python
Pillow；临时调试驱动，非产品功能）。
