# Familiar 桌面宠物素材包制作指南 (AI 像素风)

本指南介绍如何从一张普通宠物/人物图片出发，利用 AI 工具扣图提取主体、生成各状态的像素风图像，并按照 **Familiar 素材包规范 (Format Version 1)** 打包导出为 `.fpack` 文件。

---

## 一、素材包规范与文件结构

一个标准的 Familiar 素材包为 **`.fpack` 格式文件**（本质为 ZIP 压缩包），解压后的标准目录结构如下：

```
my-custom-pet/
├── pack.json         # [必需] 素材包描述文件 (Manifest)
├── preview.png       # [可选] 配置面板中的预览图 (默认回退至 idle.png)
├── idle.png          # [必需] 待命/空闲状态图片
├── thinking.png      # [可选] 思考中状态图片
├── working.png       # [可选] 执行任务/敲代码状态图片
├── happy.png         # [可选] 完成/庆祝状态图片
├── alarmed.png       # [可选] 警告/异常状态图片
└── sleeping.png      # [可选] 休眠/无活动状态图片
```

> **支持的图片格式**：静态图片支持 `.png`, `.webp`, `.svg`；动态图片支持 `.gif`, 动态 `.webp`, `.apng`。

---

## 二、描述文件规范 (`pack.json`)

在素材包根目录下创建 `pack.json` 文件，填入如下 JSON 内容：

```json
{
  "format_version": 1,
  "id": "my-custom-pet",
  "name": "我的像素小狗",
  "author": "开发者小明",
  "created_at": "2026-08-01",
  "email": "xiaoming@example.com",
  "version": "1.0.0",
  "description": "基于个人宠物照片生成的 AI 像素风桌面宠物",
  "preview": "idle.png",
  "states": {
    "idle": "idle.png",
    "thinking": "thinking.png",
    "working": "working.png",
    "happy": "happy.png",
    "alarmed": "alarmed.png",
    "sleeping": "sleeping.png",
    "celebrating": "happy.png",
    "watching": "thinking.png"
  }
}
```

### 字段说明：
- `format_version`: **[必需]** 素材格式版本号，当前版本为 `1`。
- `id`: **[必需]** 素材包唯一标识符（建议全小写，中划线分隔，如 `my-cat-pack`）。
- `name`: **[必需]** 显示在设置面板中的素材包名称。
- `author`: **[必需]** 作者姓名或昵称。
- `created_at`: 创作日期 (YYYY-MM-DD)。
- `email`: 联系邮箱。
- `version`: 素材包版本（如 `1.0.0`）。
- `description`: 素材包简介描述。
- `preview`: 预览图文件名。
- `states`: **[必需]** 状态与图片文件映射表。必须至少包含 `idle` 键。

---

## 三、AI 素材生成与处理全流程

### 步骤 1：主体提取 (移除背景)
从输入的实拍图片中提取宠物主体，保留透明通道 (Alpha Channel)：
- **命令行工具 (推荐)**：使用 `rembg`
  ```bash
  pip install rembg
  rembg i input_pet.jpg subject.png
  ```
- **图形工具**：Photoshop「快速选择工具」->「选择主体」->「抠图导出透明 PNG」。
- **在线工具**：使用 Remove.bg 或 Removebg 在线工具。

---

### 步骤 2：AI 像素风格重绘与状态变体生成

将提取出的主体 `subject.png` 导入 AI 绘画工具（Midjourney, Stable Diffusion 或 ComfyUI），生成不同状态的像素风图像：

#### 常用 AI Prompt 模板 (以 Midjourney / SD 为例)

1. **基础 Prompt**：
   > `pixel art, 16-bit pixel art style, sprite sheet element, transparent background, isolated subject, [Subject Description]`

2. **状态变体 Prompt 提示词**：
   - **`idle` (待命状态)**:
     `... standing naturally, looking forward, cute, pixel art, transparent background`
   - **`working` (敲代码/忙碌中)**:
     `... sitting at a tiny desk, typing furiously on a mechanical keyboard, laptop, sparks, focused expression, pixel art`
   - **`thinking` (思考中)**:
     `... hand on chin, looking up thoughtfully, small question mark bubble overhead, pixel art`
   - **`happy` (庆祝/完成)**:
     `... jumping joyfully, cheering with arms up, sparkles, party confetti, happy face, pixel art`
   - **`alarmed` (异常/警告)**:
     `... shocked pose, sweat drop, exclamation mark overhead, wide open eyes, pixel art`
   - **`sleeping` (休眠)**:
     `... curled up peacefully, eyes closed, small Zzz bubbles, pixel art`

---

### 步骤 3：像素化后处理与尺寸规范

1. **分辨率调整**：
   将 AI 生成的各状态图片统一缩放至 **128x128** 或 **256x256** 像素。使用邻近插值（Nearest Neighbor）保持像素边缘不模糊。
2. **透明度检查**：
   确保图片的背景完全透明（无杂色像素），避免桌面宠物周围出现不必要的硬方块。

---

## 四、打包导出为 `.fpack`

将整理好的素材文件与 `pack.json` 放入同一文件夹，在终端中切入该文件夹执行打包：

```bash
# 进入素材包文件夹
cd my-custom-pet

# 压缩为 fpack 归档
zip -r ../my-custom-pet.fpack pack.json *.png
```

---

## 五、在 Familiar 中导入与验证

1. 启动 **Familiar** 桌面应用。
2. 点击托盘或右键菜单中的 **【设置】**（Settings）。
3. 切换至 **【偏好】->【宠物素材】** 栏目。
4. 点击 **【+ 导入素材包】** 按钮，选择刚才生成的 `my-custom-pet.fpack` 文件。
5. 导入成功后，在列表卡片中点击 **【使用此素材包】** 即可实时切换桌面宠物！
