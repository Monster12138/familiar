# Familiar 资源占用优化方案

> 版本：v1.0
> 更新时间：2026-08-04
> 适用范围：Familiar macOS Tauri 桌面应用

## 1. 背景与目标

Familiar 是常驻桌面的应用，空闲资源占用会直接影响设备续航和长期使用体验。本方案基于对当前发布版进程的运行采样，优先优化无状态变化时仍在重复执行的后台工作，同时保持 Agent 状态展示的实时性。

优化目标：

- 空闲、Dashboard 隐藏时，CPU 平均占用低于 1%。
- 空闲、Dashboard 显示时，CPU 平均占用低于 1.5%。
- 空闲 CPU 峰值尽量控制在 3% 以下。
- Agent 状态变化到 UI 更新的延迟不超过 500ms。
- 物理内存维持在 45–60 MiB，长时间运行时无持续增长。
- 设置窗口和资源面板不可见时，不执行对应的 UI 刷新工作。

## 2. 当前基线

2026-08-04 对 `/Applications/Familiar.app/Contents/MacOS/familiar-app` 进行只读采样。采样时进程已连续运行约 19 小时。

| 指标 | 采样结果 |
|---|---:|
| 最近 12 秒 CPU 平均值 | 约 2.6% |
| 最近 12 秒 CPU 范围 | 0%–6.4% |
| 启动以来折算 CPU 平均值 | 约 0.9% |
| 物理内存 footprint | 约 45 MiB |
| 历史物理内存峰值 | 约 56 MiB |
| RSS | 约 90 MiB |
| 线程数 | 约 29–31 |

RSS 包含共享映射和可回收页面，不应视为应用的独占内存。macOS `footprint` 给出的 45 MiB 更适合衡量实际物理内存压力。进程运行 19 小时后内存仍接近历史峰值以下，当前没有明显的持续泄漏信号。

## 3. 已识别热点

### 3.1 高频磁盘枚举

`app/src/stats.js` 每两秒调用一次 `get_system_stats`。该命令刷新 CPU 和内存的同时，通过 `Disks::new_with_refreshed_list()` 重新枚举磁盘。

macOS 采样栈显示，调用会进入 APFS、CacheDelete 和 IOKit 的卷容量查询。磁盘容量变化频率远低于 CPU 和内存，因此每两秒完整枚举磁盘没有必要，是当前最明确的周期性 CPU 热点。

### 3.2 固定周期推送完整状态

`app/src-tauri/src/main.rs` 每 500ms 获取、克隆、过滤并序列化完整的 `RenderState`，随后通过 Tauri IPC 推送到 WebView。即使 Agent 状态没有变化，这条链路也会持续运行。

### 3.3 固定周期读取配置文件

同一条 500ms 状态推送链路会调用 `load_config()`。每次调用都会检查配置路径、读取和解析 TOML，并重新创建隐藏 session 集合。这些工作应该只在启动或配置修改时发生。

### 3.4 不可见 UI 仍可能刷新

Dashboard 的系统统计使用固定 `setInterval`，没有根据组件可见性暂停。设置窗口也可能在存在但不可见时继续接收状态。隐藏 UI 的后台工作会造成不必要的采样、IPC 和 DOM 更新。

## 4. 优化设计

### 4.1 第一阶段：系统资源采样

这是风险最低、收益最明确的优化，应优先实施。

#### 后端改造

- 用一个持久化状态保存 `System`、`Disks` 和磁盘统计缓存。
- CPU、内存允许每两秒刷新。
- 磁盘信息最多每 60 秒刷新一次。
- 两次磁盘刷新之间直接返回缓存值。
- 避免前一次采样未完成时重复启动新采样。
- 初始化时只刷新真正需要的组件，避免无差别调用 `System::new_all()` 和 `refresh_all()`。

建议的数据结构：

```rust
struct SystemStatsState {
    system: System,
    disks: Disks,
    cached_disk_used: u64,
    cached_disk_total: u64,
    last_disk_refresh: Instant,
}
```

#### 前端改造

- Dashboard 显示时立即采样一次，随后每两秒刷新 CPU 和内存。
- Dashboard 隐藏时清除定时器。
- Dashboard 恢复显示时立即刷新，不等待下一个周期。
- 使用执行中标记，避免慢查询导致多个 `invoke` 重叠。
- 页面卸载时释放定时器和事件监听器。

预期收益：消除周期性的 APFS 磁盘枚举，降低当前 3%–6% 的 CPU 峰值。

### 4.2 第二阶段：运行时配置缓存

配置文件继续作为持久化来源，但应用运行期间以内存状态为准。

建议引入共享配置状态：

```rust
struct AppConfigState {
    config: RwLock<FamiliarConfig>,
    hidden_sessions: RwLock<HashSet<String>>,
    revision: AtomicU64,
}
```

改造要求：

- 应用启动时读取一次配置并初始化共享状态。
- `get_config` 返回内存快照。
- `save_config` 成功写盘后同步更新内存配置、隐藏 session 集合和 revision。
- 状态过滤直接读取缓存的隐藏 session 集合。
- 配置变化时主动重新生成并推送过滤后的状态。
- 普通状态推送路径不得读取配置文件。
- 保持旧配置文件的反序列化兼容性，并为运行时缓存增加聚焦测试。

预期收益：消除每秒两次的配置路径检查、TOML I/O、解析及重复集合分配。

### 4.3 第三阶段：按变化推送状态

建议分两步实施，以降低时序行为变化带来的风险。

#### 第一步：revision 变更检测

为状态机增加单调递增的 revision：

```rust
struct StateMachine {
    render_state: Arc<RwLock<RenderState>>,
    revision: Arc<AtomicU64>,
}
```

- 只有 `RenderState` 实际变化时才递增 revision。
- 推送任务保存上一次已发送的 revision。
- revision 未变化时不克隆、过滤、序列化或发送状态。
- 主窗口和设置窗口分别记录最后一次已发送状态。
- 初期保留 500ms 检查周期，先验证功能和性能收益。

#### 第二步：事件驱动推送

- 状态机通过 Tokio `watch` channel 发布最新 revision 或状态。
- Tauri 推送任务等待状态变化，不再固定周期轮询。
- 配置变化通过现有 `config_changed` 链路触发重新过滤和推送。
- 可保留约 30 秒一次的低频校准，作为窗口重建等场景的兜底。

预期收益：空闲时基本消除状态克隆、序列化和 WebView IPC 开销，同时保持活跃状态的更新速度。

### 4.4 第四阶段：窗口与页面生命周期

优化原则是不可见 UI 不持续消耗 UI 更新资源。

- 设置窗口不存在或不可见时，不发送 `settings_state_changed`。
- Dashboard 隐藏时停止系统统计采样。
- 桌宠隐藏时停止图片动画和 Bubble DOM 更新，但继续维护后端 Agent 状态。
- 应用进入后台或屏幕锁定后降低非关键刷新频率。
- 恢复显示时立即同步最新状态。
- 检查重复打开和关闭窗口是否会重复注册定时器或事件监听器。

该阶段涉及平台窗口行为，需要在 macOS 上进行交互验证，并保留非 macOS 平台的现有行为。

### 4.5 内存策略

当前内存不是首要问题，不建议为了少量内存收益替换 WKWebView 或改变前端架构。只实施以下低风险措施：

- 切换 sprite 时释放旧图片引用。
- `PixelSpriteRenderer.destroy()` 时清空图片 `src`、Canvas 尺寸及上下文引用。
- 状态未变化时不重复构建 DOM。
- 确认设置窗口关闭后能够释放 WebView，而不只是隐藏。
- 长时间运行时记录 footprint，验证没有单调增长。

## 5. 实施顺序

建议拆分为三个独立提交，每一步单独采样和对比：

1. `perf(stats): cache disk usage and pause hidden dashboard polling`
2. `perf(config): cache runtime config and hidden sessions`
3. `perf(state): emit render state only when changed`

窗口生命周期和内存清理可以在前三项稳定后作为第四个提交处理。不要把所有优化一次性合入，否则难以判断每项改动的实际收益和回归来源。

## 6. 验证方案

### 6.1 功能验证

- Agent 启动、思考、工作、等待和完成状态正常切换。
- celebration 和 sleep timeout 行为不变。
- 隐藏 session 后立即从桌宠和设置页面消失。
- 配置保存、重启加载及旧配置反序列化正常。
- Dashboard 打开后立即显示数据，隐藏后停止采样。
- 设置窗口反复打开和关闭不产生重复刷新。
- macOS 桌宠窗口的拖动、置顶和跨桌面行为正常。

### 6.2 自动化验证

- 为磁盘缓存增加时间边界和刷新行为测试。
- 为运行时配置状态增加加载、保存和缓存同步测试。
- 为状态 revision 增加“实际变化才递增”的测试。
- 为旧配置文件反序列化和 TOML 序列化增加回归测试。
- 运行受影响包的聚焦 Rust 测试。
- 运行 `npm run build` 验证前端构建。
- 对所有修改文件运行 `git diff --check`。

### 6.3 性能验证

每个优化提交使用相同条件重复采样：

1. 启动发布构建并等待状态稳定。
2. Dashboard 隐藏，连续采样 CPU 60 秒。
3. Dashboard 显示，连续采样 CPU 60 秒。
4. 记录当前和峰值 physical footprint。
5. 模拟多个 Agent 高频事件，记录 CPU 峰值和 UI 延迟。
6. 保持运行至少 8 小时，检查内存趋势。

建议记录平均值、中位数、P95 和最大值，避免用单点 CPU 读数判断优化效果。

## 7. 验收标准

| 场景 | CPU 目标 | 内存目标 | 更新延迟 |
|---|---:|---:|---:|
| 空闲、Dashboard 隐藏 | 平均 < 1% | ≤ 60 MiB | Agent 状态 ≤ 500ms |
| 空闲、Dashboard 显示 | 平均 < 1.5% | ≤ 65 MiB | 系统统计 ≤ 2s |
| Agent 高频活动 | 峰值尽量 < 8% | 无持续增长 | Agent 状态 ≤ 500ms |
| 连续运行 8 小时 | 无持续 CPU 异常 | 峰值 ≤ 70 MiB 且无上升趋势 | 无功能退化 |

若目标未达到，应分别关闭系统统计、状态推送和设置窗口进行对照采样，继续定位剩余热点，而不是提前进行大范围架构调整。

## 8. 风险与回退

- 磁盘缓存可能使容量显示最多延迟 60 秒；对桌面状态面板属于可接受范围。
- 配置缓存必须保证保存成功后才更新运行时状态，避免内存和磁盘配置不一致。
- revision 必须覆盖定时清理、sleep timeout 和配置过滤变化，否则可能遗漏 UI 更新。
- 页面可见性在 Tauri 窗口隐藏场景下可能与浏览器 `document.visibilityState` 不完全一致，应结合 Tauri 窗口事件验证。
- 每个阶段保持独立提交；发生回归时可以单独回退，不影响其他优化。
