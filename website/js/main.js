// Familiar landing page — lightweight bilingual toggle (no build tooling).
// Text lives in `translations`; elements opt in via data-i18n="key". The
// choice is persisted in localStorage and defaults to zh-CN, matching the app.

const translations = {
  'zh-CN': {
    nav_features: '特性',
    nav_integrations: '集成',
    nav_privacy: '隐私',
    nav_download: '下载',
    hero_tagline: '一只会回应你工作世界的桌面宠物。',
    hero_sub: '本地优先的编程 Agent 桌面伴侣：把 Hook 活动转化为像素风桌宠的细微回应，不打扰你的工作流。',
    btn_download: '下载 Familiar',
    btn_source: 'GitHub 仓库',
    hero_note: 'macOS · Windows · Linux 支持开发中，最新版本见 GitHub Releases。',
    features_title: '特性',
    feat_pet_title: '会回应的桌面宠物',
    feat_pet_desc: '像素风动画随工作活动变化，安静地陪伴而不打扰你的工作空间。',
    feat_agent_title: 'Agent 状态联动',
    feat_agent_desc: '连接 Claude Code、Codex CLI、Qoder 和 Google Antigravity，让桌宠反映它们的活动。',
    feat_hook_title: 'Hook 扩展能力',
    feat_hook_desc: '通过同一套事件流水线接入其他 Agent 和本地工作流，也支持非编程场景。',
    feat_modular_title: '模块化架构',
    feat_modular_desc: 'Hook 解析、状态管理、传输和渲染彼此分层，职责清晰。',
    feat_stats_title: '本机状态视图',
    feat_stats_desc: '在桌宠旁显示当前 CPU、内存和磁盘状态，一眼掌握机器负载。',
    feat_sprite_title: 'Sprite Pack 皮肤包',
    feat_sprite_desc: '使用文档化的 .fpack 格式导入和打包更多桌宠形象。',
    feat_privacy_title: '隐私优先',
    feat_privacy_desc: 'Hook 数据只在本机处理，运行日志保持最小化，不包含远程遥测。',
    integrations_title: '支持的集成',
    th_integration: '集成',
    th_transport: '传输方式',
    th_status: '状态',
    tr_claude_transport: '通过本地 Familiar 通道上报 Hook 事件',
    tr_codex_transport: '通过本地 Familiar 通道上报 Hook 事件',
    tr_qoder_transport: '通过本地 Familiar 通道上报 Hook 事件',
    tr_antigravity_transport: '支持 transcript 提取的原生 Hook 适配器',
    status_pending: '待验证',
    status_available: '可用',
    perf_title: '性能表现',
    perf_desc: '对应 UI 隐藏时暂停轮询，只发送实际变化的渲染状态。Apple Silicon macOS 上连续运行约 2 小时 39 分钟后的 30 秒空闲采样：',
    stat_cpu: '平均 CPU 占用',
    stat_cpu_median: 'CPU 中位数',
    stat_mem: '物理内存占用',
    stat_threads: '线程数',
    privacy_title: '隐私优先',
    privacy_text: '为了展示当前状态，Familiar 可能读取最近的任务说明、命令文本、工具名称、文件路径等本地 Hook 数据。当前版本不会持久化完整 transcript 或 Agent 事件历史，也不会把采集到的数据发送到远程服务。运行日志只包含事件类型、会话标识和状态等最小元数据。',
    privacy_text2: '完整的数据流、配置修改、日志和清理说明见项目文档。',
    btn_privacy: '查看隐私文档',
    download_title: '下载 Familiar',
    download_desc: '从 GitHub Releases 下载最新的安装包，或从源码构建。修改 Hook 配置前，程序会自动创建备份。',
    download_note: '也可访问 GitHub 仓库阅读源码、提交 Issue 或参与贡献。',
  },
  'en-US': {
    nav_features: 'Features',
    nav_integrations: 'Integrations',
    nav_privacy: 'Privacy',
    nav_download: 'Download',
    hero_tagline: 'A desktop pet that reacts to your world.',
    hero_sub: 'A local-first companion for coding agents: it turns hook activity into subtle pixel-art reactions without taking over your workspace.',
    btn_download: 'Download Familiar',
    btn_source: 'GitHub Repository',
    hero_note: 'macOS · Windows · Linux support is evolving; grab the latest release from GitHub Releases.',
    features_title: 'Features',
    feat_pet_title: 'Reactive desktop pet',
    feat_pet_desc: 'Pixel-art animations respond to activity without taking over your workspace.',
    feat_agent_title: 'Agent state linkage',
    feat_agent_desc: 'Connect Claude Code, Codex CLI, Qoder, and Google Antigravity so the companion can reflect their activity.',
    feat_hook_title: 'Hook-based extensibility',
    feat_hook_desc: 'Adapt other agents and local workflows through the same event pipeline, including non-programming scenarios.',
    feat_modular_title: 'Modular architecture',
    feat_modular_desc: 'Hook parsing, state management, transport, and rendering stay in separate layers.',
    feat_stats_title: 'Local activity view',
    feat_stats_desc: 'Show current CPU, memory, and disk state right next to the companion.',
    feat_sprite_title: 'Sprite packs',
    feat_sprite_desc: 'Import and package additional companions with the documented .fpack format.',
    feat_privacy_title: 'Privacy-first by design',
    feat_privacy_desc: 'Process hook data locally and keep logs minimal; no remote telemetry is included.',
    integrations_title: 'Supported integrations',
    th_integration: 'Integration',
    th_transport: 'Transport',
    th_status: 'Status',
    tr_claude_transport: 'Hook reporter over the local Familiar channel',
    tr_codex_transport: 'Hook reporter over the local Familiar channel',
    tr_qoder_transport: 'Hook reporter over the local Familiar channel',
    tr_antigravity_transport: 'Native hook adapter with transcript extraction',
    status_pending: 'Pending verification',
    status_available: 'Available',
    perf_title: 'Performance',
    perf_desc: 'Familiar pauses polling while the UI is hidden and only sends render state when it changes. A 30-second idle sample after ~2h39m of continuous runtime on Apple Silicon macOS:',
    stat_cpu: 'Average CPU',
    stat_cpu_median: 'Median CPU',
    stat_mem: 'Physical memory footprint',
    stat_threads: 'Threads',
    privacy_title: 'Privacy-first',
    privacy_text: 'Familiar may read the latest task description, command text, tool names, file paths, and related local hook data to render the current state. It does not persist a full transcript or agent event history, and it does not send captured data to a remote service. Operational logs contain only minimal metadata such as event kind, session identifier, and mood.',
    privacy_text2: 'See the project documentation for the full data-flow, configuration, logging, and cleanup details.',
    btn_privacy: 'Read the privacy docs',
    download_title: 'Download Familiar',
    download_desc: 'Grab the latest installer from GitHub Releases, or build from source. Existing agent configuration is backed up before it is changed.',
    download_note: 'Visit the GitHub repository to read the source, open issues, or contribute.',
  },
};

const STORAGE_KEY = 'familiar-site-lang';
const DEFAULT_LANG = 'zh-CN';

function applyLang(lang) {
  const dict = translations[lang] || translations[DEFAULT_LANG];
  document.documentElement.lang = lang === 'zh-CN' ? 'zh-CN' : 'en';
  document.querySelectorAll('[data-i18n]').forEach((el) => {
    const key = el.getAttribute('data-i18n');
    if (dict[key]) el.textContent = dict[key];
  });
  const toggle = document.getElementById('lang-toggle');
  if (toggle) toggle.textContent = lang === 'zh-CN' ? 'English' : '中文';
}

function init() {
  let lang = localStorage.getItem(STORAGE_KEY);
  if (!lang || !translations[lang]) lang = DEFAULT_LANG;
  applyLang(lang);

  const toggle = document.getElementById('lang-toggle');
  if (toggle) {
    toggle.addEventListener('click', () => {
      const next = document.documentElement.lang === 'zh-CN' ? 'en-US' : 'zh-CN';
      localStorage.setItem(STORAGE_KEY, next);
      applyLang(next);
    });
  }
}

init();
