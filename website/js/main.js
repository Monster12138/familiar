// Familiar landing page — lightweight bilingual toggle (no build tooling).
// Text lives in `translations`; elements opt in via data-i18n="key". The
// choice is persisted in localStorage and defaults to zh-CN, matching the app.

const translations = {
  'zh-CN': {
    nav_features: '特性',
    nav_integrations: '集成',
    nav_privacy: '隐私',
    nav_download: '下载',
    hero_eyebrow: '你的 Agent，终于有了表情',
    hero_tagline: '一只会回应你工作世界的桌面宠物。',
    hero_sub: '仅本地计算的 Agent 桌面伴侣：把 Agent 活动转化为可爱桌宠的一举一动。',
    btn_download: '下载 Familiar',
    btn_source: 'GitHub 仓库',
    hero_note: 'macOS · Windows · Linux 支持开发中，最新版本见 GitHub Releases。',
    demo_aria: 'Claude Code、ChatGPT 与 DeepSeek Harness 的事件汇聚到 Familiar 桌宠',
    pet_aria: 'Familiar 桌宠随 Agent 状态切换动作',
    demo_local: 'HOOK 事件 → 宠物响应',
    pet_idle: '等待事件',
    pet_thinking: '正在思考',
    pet_working: '正在工作',
    pet_done: '任务完成',
    bubble_prompt_idle: '等待下一个任务',
    bubble_prompt_thinking: '分析官网前端结构',
    bubble_prompt_working: '更新 Hero 动效',
    bubble_prompt_done: '官网更新完成',
    features_title: '特性',
    feat_pet_title: '会回应的桌面宠物',
    feat_pet_desc: '像素风动画随工作活动变化，安静陪伴不打扰。',
    feat_agent_title: 'Agent 状态联动',
    feat_agent_desc: '支持连接 Claude Code、Codex CLI、DeepSeek Harness、Qoder 与 Antigravity，桌宠实时反映 Agent 活动。',
    feat_privacy_title: '隐私优先',
    feat_privacy_desc: '数据只在本机处理，无遥测，运行日志最小化。',
    feat_stats_title: '支持远程连接',
    feat_stats_desc: '订阅远程服务器状态，让本地桌宠同步呈现远端 Agent 活动。',
    feat_perf_title: '轻量低占用',
    feat_perf_desc: 'UI 隐藏时暂停轮询；空闲平均 CPU 0.013%，内存约 37 MiB。',
    feat_sprite_title: 'Sprite Pack 皮肤包',
    feat_sprite_desc: '用 .fpack 格式导入和打包更多桌宠形象。',
    integrations_title: '支持的集成',
    th_integration: '集成',
    th_transport: '传输方式',
    th_status: '状态',
    status_available: '可用',
    perf_title: '性能表现',
    perf_desc: 'Apple Silicon macOS 上连续运行约 2 小时 39 分钟后的 30 秒空闲采样：',
    stat_cpu: '平均 CPU 占用',
    stat_mem: '物理内存占用',
    privacy_title: '隐私优先',
    privacy_text: '为了展示当前状态，Familiar 可能读取最近的任务说明、命令文本、工具名称、文件路径等本地 Hook 数据。仅用于本地计算，不会持久化 Agent 事件历史，也不会把采集到的数据发送到远程服务。',
    privacy_text2: '完整的数据流、配置修改、日志和清理说明见项目文档。',
    btn_privacy: '查看隐私文档',
    download_title: '下载 Familiar',
    download_desc: '从 GitHub Releases 下载最新的安装包，或从源码构建。',
    download_note: '也可访问 GitHub 仓库阅读源码、提交 Issue 或参与贡献。',
  },
  'en-US': {
    nav_features: 'Features',
    nav_integrations: 'Integrations',
    nav_privacy: 'Privacy',
    nav_download: 'Download',
    hero_eyebrow: 'Your agents, now with expressions',
    hero_tagline: 'A desktop pet that reacts to your world.',
    hero_sub: 'A local-only desktop companion for coding agents: it turns agent activity into every move of an adorable desktop pet.',
    btn_download: 'Download Familiar',
    btn_source: 'GitHub Repository',
    hero_note: 'macOS · Windows · Linux support is evolving; grab the latest release from GitHub Releases.',
    demo_aria: 'Events from Claude Code, ChatGPT, and DeepSeek Harness converge into the Familiar desktop pet',
    pet_aria: 'The Familiar pet changes actions with agent state',
    demo_local: 'HOOK EVENTS → PET REACTIONS',
    pet_idle: 'Waiting for events',
    pet_thinking: 'Thinking',
    pet_working: 'Working',
    pet_done: 'Task complete',
    bubble_prompt_idle: 'Waiting for the next task',
    bubble_prompt_thinking: 'Analyzing the landing page',
    bubble_prompt_working: 'Updating the hero animation',
    bubble_prompt_done: 'Website update complete',
    features_title: 'Features',
    feat_pet_title: 'Reactive desktop pet',
    feat_pet_desc: 'Pixel-art animations respond to your work without interrupting your flow.',
    feat_agent_title: 'Agent state linkage',
    feat_agent_desc: 'Supports Claude Code, Codex CLI, DeepSeek Harness, Qoder, and Antigravity so the pet mirrors agent activity in real time.',
    feat_privacy_title: 'Privacy-first',
    feat_privacy_desc: 'Data is processed locally only; no telemetry, minimal logs.',
    feat_stats_title: 'Remote connections',
    feat_stats_desc: 'Subscribe to a remote server so the local pet reflects remote agent activity.',
    feat_perf_title: 'Lightweight',
    feat_perf_desc: 'Polling pauses while the UI is hidden; ~0.013% idle CPU and ~37 MiB memory.',
    feat_sprite_title: 'Sprite packs',
    feat_sprite_desc: 'Import and package more companions with the .fpack format.',
    integrations_title: 'Supported integrations',
    th_integration: 'Integration',
    th_transport: 'Transport',
    th_status: 'Status',
    status_available: 'Available',
    perf_title: 'Performance',
    perf_desc: 'A 30-second idle sample after ~2h39m of continuous runtime on Apple Silicon macOS.',
    stat_cpu: 'Average CPU',
    stat_mem: 'Physical memory footprint',
    privacy_title: 'Privacy-first',
    privacy_text: 'Familiar may read the latest task description, command text, tool names, file paths, and related local hook data to render the current state. It is used for local computation only — it does not persist agent event history, and it does not send captured data to a remote service.',
    privacy_text2: 'See the project documentation for the full data-flow, configuration, logging, and cleanup details.',
    btn_privacy: 'Read the privacy docs',
    download_title: 'Download Familiar',
    download_desc: 'Grab the latest installer from GitHub Releases, or build from source.',
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
  document.querySelectorAll('[data-i18n-aria]').forEach((el) => {
    const key = el.getAttribute('data-i18n-aria');
    if (dict[key]) el.setAttribute('aria-label', dict[key]);
  });
  document.querySelectorAll('[data-i18n-alt]').forEach((el) => {
    const key = el.getAttribute('data-i18n-alt');
    if (dict[key]) el.setAttribute('alt', dict[key]);
  });
  const activeState = document.getElementById('pet-sprite')?.dataset.state;
  if (activeState) {
    const status = document.getElementById('pet-status');
    const prompt = document.getElementById('bubble-prompt');
    if (status) status.textContent = dict[`pet_${activeState}`];
    if (prompt) prompt.textContent = dict[`bubble_prompt_${activeState}`];
  }
  const toggle = document.getElementById('lang-toggle');
  if (toggle) toggle.textContent = lang === 'zh-CN' ? 'English' : '中文';
}

function initPetDemo() {
  const sprite = document.getElementById('pet-sprite');
  const status = document.getElementById('pet-status');
  const prompt = document.getElementById('bubble-prompt');
  const badge = document.getElementById('bubble-badge');
  const spinner = document.getElementById('bubble-spinner');
  if (!sprite || !status || !prompt || !badge || !spinner) return;

  const states = ['idle', 'thinking', 'working', 'done'];
  const stateFiles = {
    idle: 'idle.png',
    thinking: 'thinking.png',
    working: 'working.png',
    done: 'celebrating.png',
  };
  const stateSources = ['ChatGPT', 'Claude', 'DSH', 'ChatGPT'];
  let index = 0;
  const showState = () => {
    const state = states[index];
    sprite.dataset.state = state;
    sprite.src = `assets/tabby-cat/${stateFiles[state]}`;
    const lang = document.documentElement.lang === 'zh-CN' ? 'zh-CN' : 'en-US';
    status.textContent = translations[lang][`pet_${state}`];
    prompt.textContent = translations[lang][`bubble_prompt_${state}`];
    badge.textContent = stateSources[index];
    badge.dataset.source = stateSources[index].toLowerCase();
    spinner.dataset.complete = state === 'done' ? 'true' : 'false';
    index = (index + 1) % states.length;
  };
  showState();
  window.setInterval(showState, 1800);
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
  initPetDemo();
}

init();
