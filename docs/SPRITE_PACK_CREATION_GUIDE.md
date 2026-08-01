# Familiar 桌面宠物素材包制作指南 (AI 像素风)

本指南介绍如何从一张普通宠物/人物图片出发，利用 AI 工具扣图提取主体、生成各状态的像素风图像，并按照 **Familiar 素材包规范 (Format Version 1)** 打包导出为 `.fpack` 文件。

---

## 一、素材包规范与文件结构

一个标准的 Familiar 素材包为 **`.fpack` 格式文件**（本质为 ZIP 压缩包），解压后的标准目录结构如下：

```
my-custom-pet/
├── pack.json         # [必需] 素材包描述文件 (Manifest)
├── LICENSE           # [推荐] 素材的完整许可证文本
├── preview.png       # [可选] 配置面板中的预览图 (默认回退至 idle.png)
├── idle.png          # [必需] 待命/空闲状态图片
├── working.png       # [可选] 执行任务/敲代码状态图片
├── thinking.png      # [可选] 思考中状态图片
├── interacting.png   # [可选] 互动中状态图片 (鼠标点击交互)
├── celebrating.png   # [可选] 任务完成/庆祝状态图片
├── alarmed.png       # [可选] 警告/异常报错状态图片
├── sleeping.png      # [可选] 休眠/无活动状态图片
└── watching.png      # [可选] 观察/等待用户输入状态图片
```

> **支持的图片格式**：静态图片支持 `.png`, `.webp`, `.svg`；动态图片支持 `.gif`, 动态 `.webp`, `.apng`。

---

## 二、描述文件规范 (`pack.json`)

在素材包根目录下创建 `pack.json` 文件，填入如下 JSON 内容：

```json
{
  "format_version": 1,
  "id": "my-custom-pet",
  "name": "我的像素小猫",
  "author": "开发者小明",
  "created_at": "2026-08-01",
  "email": "xiaoming@example.com",
  "version": "1.0.0",
  "license": "CC-BY-4.0",
  "source": "https://example.com/my-custom-pet",
  "description": "基于个人宠物照片生成的 AI 极简像素风桌面宠物",
  "preview": "idle.png",
  "states": {
    "idle": "idle.png",
    "working": "working.png",
    "thinking": "thinking.png",
    "interacting": "interacting.png",
    "celebrating": "celebrating.png",
    "alarmed": "alarmed.png",
    "sleeping": "sleeping.png",
    "watching": "watching.png"
  },
  "sha256": "6bfad07af8469b061b4d44a3acb0c567b71f96238628c1ddcfaa34373ecee1ce"
}
```

### 字段说明：
- `format_version`: **[必需]** 素材格式版本号，当前版本为 `1`。
- `id`: **[必需]** 素材包唯一标识符（建议全小写，中划线分隔，如 `tabby-cat`）。
- `name`: **[必需]** 显示在设置面板中的素材包名称。
- `author`: **[必需]** 作者姓名或昵称。
- `created_at`: 创作日期 (YYYY-MM-DD)。
- `email`: 联系邮箱。
- `version`: 素材包版本（如 `1.0.0`）。
- `license`: **[强烈推荐]** 素材的 SPDX 许可证表达式（如 `CC-BY-4.0`、`CC0-1.0` 或 `MIT OR Apache-2.0`）。
- `source`: **[强烈推荐]** 素材主页或可核验的原始来源链接。
- `description`: 素材包简介描述。
- `preview`: 预览图文件名。
- `states`: **[必需]** 状态与图片文件映射表。必须至少包含 `idle` 键。
- `sha256`: **[可选]** 素材包除了 `pack.json` 之外所有资源文件的 sha256 校验和。

---

## 三、AI 素材生成与风格指导

### 1. 官方推荐风格：极简 2D 复古像素风 (Minimal Retro Pixel Style)

官方默认素材（如 `british-blue` 皮蛋、`tabby-cat` 小虎）采用的是**极简复古 2D 像素风格**，具有高对比度、清晰可爱、占用分辨率小等显著优点。

#### 核心视觉特征：
- **粗黑像素外描边 (Bold Black Outline)**：1-2 像素宽度的黑色描边包裹角色轮廓，确保角色的任意动态在各种不同桌面壁纸（深色/浅色/复杂壁纸）上都能极高辨识度凸显。
- **大圆眼/经典复古眼 (Big Round Retro Eyes)**：大面积圆形白色眼眶结合黑色小眼珠，形象生动讨喜。
- **平铺色块 (Flat Color Palette)**：减少复杂的渐变与光影细节，使用少色彩的平铺色块填充，保持极简感。
- **闭合描边防透光**：确保主体描边闭合，方便在抠图阶段将内部白色（如白下巴、白脚掌、Zzz 气泡）完整保留。

---

### 2. Prompt 提示词工程模板

在生成极简像素风素材时，建议使用以下 Prompt 结构：

#### 基础通用 Prompt 模板：
> `Minimal 2D pixel art game character, strictly in retro minimalist pixel sprite style, bold black pixel outline, big cute round white eyes, clean minimal pixel sprite, isolated sprite, solid white background, [Subject Features]`

#### 状态变体 Prompt 提示词：

1. **`idle` (待命/空闲)**:
   > `Minimal 2D pixel art character of [Subject Description] with thick black outline and big round eyes, standing naturally, clean minimal pixel sprite, solid white background`

2. **`working` (敲代码/工作)**:
   > `Same minimal 2D pixel art character of [Subject Description] with thick black outline. Sitting at a small laptop typing on keyboard, focused pose, clean minimal pixel sprite, solid white background`

3. **`thinking` (思考中)**:
   > `Same minimal 2D pixel art character of [Subject Description] with thick black outline. Paw on chin looking up thoughtfully, small question mark bubble overhead, clean minimal pixel sprite, solid white background`

4. **`happy` (庆祝/成功)**:
   > `Same minimal 2D pixel art character of [Subject Description] with thick black outline. Standing on hind legs cheering happily with paws up, sparkles around, happy face, clean minimal pixel sprite, solid white background`

5. **`alarmed` (异常/警告)**:
   > `Same minimal 2D pixel art character of [Subject Description] with thick black outline. Shocked expression, wide eyes, sweat drop on head, clean minimal pixel sprite, solid white background`

6. **`sleeping` (休眠)**:
   > `Same minimal 2D pixel art character of [Subject Description] with thick black outline. Curled up sleeping peacefully, eyes closed, small Zzz speech bubble, clean minimal pixel sprite, solid white background`

> **提示词技巧**：在生成 2 - 6 种状态时，一定要将 `idle.png` 作为 Image Reference 输入给 AI 模型，并在 Prompt 开头加上 `"Same minimal 2D pixel art character of ..."` 保持角色的统一性。

---

## 四、图像后处理与背景抠图 (BFS 连通填充)

### 1. 透明背景处理要点
- 绝对**不要使用简单的全局阈值过滤（Global Thresholding）**，否则角色体内的白色部位（白脚掌、白下巴、`Zzz` 气泡填色）会被误扣成透明孔。
- 应该使用**边缘连通泛滥填充算法 (BFS / DFS Flood Fill)** 或专业的背景提取算法（如 `rembg`），从图像外围四角（0, 0）开始遍历填充，遇到黑色描边时终止，只剥离纯外围背景。

```python
# Python BFS 示例：仅消除外围背景，保留角色内部白色部位
import collections
from PIL import Image

def remove_outer_background(image_path, save_path):
    img = Image.open(image_path).convert('RGBA')
    width, height = img.size
    pixels = img.load()

    def is_white(r, g, b):
        return r > 230 and g > 230 and b > 230

    visited = set()
    queue = collections.deque()

    # 将图片四条边缘的白色像素入队
    for x in range(width):
        for y in (0, height - 1):
            if is_white(*pixels[x, y][:3]) and (x, y) not in visited:
                visited.add((x, y))
                queue.append((x, y))
    for y in range(height):
        for x in (0, width - 1):
            if is_white(*pixels[x, y][:3]) and (x, y) not in visited:
                visited.add((x, y))
                queue.append((x, y))

    # BFS 广度优先搜索泛滥填充
    while queue:
        cx, cy = queue.popleft()
        # 仅将最外层连通背景的 Alpha 设为 0
        pixels[cx, cy] = (0, 0, 0, 0)

        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            nx, ny = cx + dx, cy + dy
            if 0 <= nx < width and 0 <= ny < height and (nx, ny) not in visited:
                if is_white(*pixels[nx, ny][:3]):
                    visited.add((nx, ny))
                    queue.append((nx, ny))

    img.save(save_path, 'PNG')
```

---

## 五、打包导出为 `.fpack`

整理好 `pack.json` 和所有的图片资源后，在终端中执行打包命令：

```bash
# 进入素材包文件夹
cd my-custom-pet

# 压缩为 fpack 归档
zip -r ../my-custom-pet.fpack pack.json LICENSE* *.png
```

---

## 六、在 Familiar 中导入与验证

1. 启动 **Familiar** 桌面应用。
2. 点击应用面板顶部的 **【设置】**。
3. 切换至 **【偏好】->【宠物素材】** 栏目。
4. 点击 **【导入素材包】图标**，选择刚才生成的 `my-custom-pet.fpack` 文件。
5. 点击 **【打开文件夹】图标** 可随时打开存放素材的本地目录 (`~/.config/familiar/sprites`) 进行手动管理。
