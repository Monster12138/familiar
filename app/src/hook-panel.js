// Shared mountable hook management panel. Used by both the onboarding
// second step and the settings panel: `mountHookPanel(container)` renders
// the full agent hook UI (inject / uninstall / view-config / config path /
// hook-point details / test) into any container, injecting the required
// modals into the page on first mount. Reuses diff.js and i18n.js.

import { applyTranslations, t } from './i18n.js';
import { renderDiffRows, syntaxHighlightJSON } from './diff.js';

const { invoke } = window.__TAURI__.core;

// The three modals used by the panel, injected into document.body once on
// first mount so host pages do not need to carry the markup.
const HOOK_MODALS_HTML = `
    <div id="hook-modal" class="modal-overlay" style="display:none;">
        <div class="modal-content diff-modal-content">
            <h3 data-i18n="modal_title_preview">确认注入 Hooks 契约</h3>
            <p data-i18n="modal_desc_preview" style="color:var(--text-muted);font-size:13px;margin: 8px 0 16px;">以下配置将会合并到 Agent 的配置文件中。我们将自动生成一个 <code>.bak</code> 备份以防万一。</p>

            <div class="diff-viewer">
                <div class="diff-pane">
                    <div class="diff-header">Before</div>
                    <pre><code id="inject-before-code" class="code-preview"></code></pre>
                </div>
                <div class="diff-pane">
                    <div class="diff-header">After</div>
                    <pre><code id="inject-after-code" class="code-preview"></code></pre>
                </div>
            </div>

            <div class="modal-path-bar" id="inject-path-bar" style="display:none;">
                <div class="modal-path-text" id="inject-path-text"></div>
                <button class="modal-path-copy" data-target="inject-path-text" data-i18n="btn_copy">复制</button>
            </div>

            <div class="modal-actions">
                <button class="secondary-btn" id="btn-modal-cancel" data-i18n="btn_cancel">取消</button>
                <button class="primary-btn" id="btn-modal-confirm" data-i18n="btn_confirm_inject">确认</button>
            </div>
        </div>
    </div>

    <div id="uninstall-modal" class="modal-overlay" style="display:none;">
        <div class="modal-content diff-modal-content">
            <h3 data-i18n="modal_title_uninstall">确认卸载 Hooks</h3>
            <p data-i18n="confirm_uninstall" style="color:var(--text-muted);font-size:13px;margin: 8px 0 16px;">卸载后该 Agent 的状态将不再同步给桌面宠物，是否确认？</p>

            <div class="diff-viewer">
                <div class="diff-pane">
                    <div class="diff-header">Before</div>
                    <pre><code id="uninstall-before-code" class="code-preview"></code></pre>
                </div>
                <div class="diff-pane">
                    <div class="diff-header">After</div>
                    <pre><code id="uninstall-after-code" class="code-preview"></code></pre>
                </div>
            </div>

            <div class="modal-path-bar" id="uninstall-path-bar" style="display:none;">
                <div class="modal-path-text" id="uninstall-path-text"></div>
                <button class="modal-path-copy" data-target="uninstall-path-text" data-i18n="btn_copy">复制</button>
            </div>

            <div class="modal-actions">
                <button class="secondary-btn" id="btn-uninstall-cancel" data-i18n="btn_cancel">取消</button>
                <button class="danger-btn" id="btn-uninstall-confirm" data-i18n="btn_uninstall" style="padding: 10px 20px; font-size: 14px; font-weight: 500;">确认卸载</button>
            </div>
        </div>
    </div>

    <div id="config-viewer-modal" class="modal-overlay" style="display:none;">
        <div class="modal-content diff-modal-content">
            <h3 data-i18n="modal_title_view_config">查看配置</h3>
            <pre style="flex: 1; overflow: auto; margin-bottom: 16px;"><code id="config-viewer-code" class="code-preview" style="height: 100%; box-sizing: border-box;"></code></pre>

            <div class="modal-path-bar" id="config-viewer-path-bar" style="display:none;">
                <div class="modal-path-text" id="config-viewer-path-text"></div>
                <button class="modal-path-copy" data-target="config-viewer-path-text" data-i18n="btn_copy">复制</button>
            </div>

            <div class="modal-actions">
                <button class="primary-btn" id="btn-config-viewer-close" data-i18n="btn_close">关闭</button>
            </div>
        </div>
    </div>`;

// Built-in (fallback) status per event kind, mirroring StateMachine::apply_event.
// `null` means the event is a no-op by default (keeps the agent's current status).
const EVENT_DEFAULT_STATUS = {
    'AgentStarted': 'working',
    'Thinking': 'thinking',
    'Processing': 'working',
    'ReadingFile': 'working',
    'WritingFile': 'working',
    'RunningCommand': 'working',
    'SearchingCode': 'working',
    'BrowsingWeb': 'working',
    'TaskCompleted': 'completed',
    'TaskFailed': 'failed',
    'WaitingForInput': 'pending',
    'SubagentStarted': null,
    'SubagentStopped': null,
};

const AGENT_DISPLAY = {
    'antigravity': 'Antigravity',
    'claude-code': 'Claude Code',
    'codex': 'Codex',
    'deepseek-harness': 'DeepSeek Harness',
    'qoder': 'Qoder',
};

const AGENTS = ['antigravity', 'claude-code', 'codex', 'deepseek-harness', 'qoder'];

let panelContainer = null;
let currentLang = 'zh-CN';
let currentConfig = {};
let currentInjectingAgent = null;
let currentUninstallingAgent = null;
let hooksStatusCache = {};
let hookDetailsCache = {};      // agent -> AgentHookDetail (lazy)
let expandedAgents = new Set(); // track which agent cards are expanded

// Modal DOM refs, captured after the modals are injected on first mount.
let hookModal;
let btnModalConfirm;
let injectBeforeCode;
let injectAfterCode;
let uninstallModal;
let btnUninstallConfirm;
let uninstallBeforeCode;
let uninstallAfterCode;
let configViewerModal;
let configViewerCode;

// Same global toast used by the settings panel, replicated here.
let toastTimer = null;
function showToast(message, type, durationMs) {
    let toast = document.getElementById('app-toast');
    if (!toast) {
        toast = document.createElement('div');
        toast.id = 'app-toast';
        toast.className = 'app-toast';
        document.body.appendChild(toast);
    }
    toast.textContent = message;
    toast.className = 'app-toast show' + (type ? ' ' + type : '');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
        toast.className = 'app-toast';
    }, durationMs || 4000);
}

// Raw hook event name → AgentEventType kind, mirroring the adapter
// (adapter.rs map_event_type) and Antigravity (antigravity.rs
// map_native_hook) parsers. Only used to display the status a hook event
// leads to. PreToolUse's kind depends on the tool, which a hook point does
// not carry, so it resolves generically to Processing (Working by default).
function hookEventKind(eventName, agent) {
    if (agent === 'antigravity') {
        switch (eventName) {
            case 'SessionStart': return 'AgentStarted';
            case 'PreToolUse': return 'Processing';
            case 'PostToolUse': return 'Processing';
            case 'PreInvocation':
            case 'PostInvocation': return 'Thinking';
            case 'Stop':
            case 'SessionEnd': return 'TaskCompleted';
            default: return null;
        }
    }
    switch (eventName) {
        case 'SessionStart':
        case 'start':
        case 'USER_INPUT':
        case 'UserPromptSubmit': return 'AgentStarted';
        case 'Stop':
        case 'stop':
        case 'exit':
        case 'SessionEnd': return 'TaskCompleted';
        case 'StopFailure': return 'TaskFailed';
        case 'PreToolUse':
        case 'tool_call': return 'Processing';
        case 'PostToolUse':
        case 'tool_result': return 'Processing';
        case 'PostToolUseFailure': return 'Processing';
        case 'PermissionRequest': return 'WaitingForInput';
        case 'SubagentStart': return 'SubagentStarted';
        case 'SubagentStop': return 'SubagentStopped';
        default: return null;
    }
}

// Effective pet status a hook event produces: the user's override from the
// event-status mapping if set, else the built-in default (or null for the
// no-op Subagent events).
function hookEventStatus(eventName, agent) {
    const kind = hookEventKind(eventName, agent);
    if (!kind) return null;
    const map = currentConfig?.renderer?.['desktop-pet']?.event_status_map || {};
    return map[kind] || EVENT_DEFAULT_STATUS[kind] || null;
}

function renderHookPointsTable(detailEl, hookDetail, agent) {
    const points = hookDetail.hook_points || [];

    if (points.length === 0) {
        detailEl.innerHTML = `<div class="hook-detail-empty">${t('lbl_no_hook_points', currentLang)}</div>`;
        return;
    }

    let html = '<div class="hook-points-table">';
    html += '<div class="hook-points-header">';
    html += `<span class="hook-col-event">${t('lbl_hook_event', currentLang)}</span>`;
    html += `<span class="hook-col-status">${t('lbl_hook_status', currentLang)}</span>`;
    html += `<span class="hook-col-test"></span>`;
    html += '</div>';

    points.forEach(pt => {
        const matcherLabel = pt.matcher ? ` (matcher: ${pt.matcher})` : '';
        const statusKey = hookEventStatus(pt.event_name, agent);
        const statusLabel = statusKey
            ? t('status_' + statusKey.replace(/-/g, '_'), currentLang)
            : t('status_default_noop', currentLang);
        const copyCmd = pt.test_command || pt.command;
        html += '<div class="hook-point-row">';
        html += `<span class="hook-col-event"><code>${pt.event_name}</code>${matcherLabel}</span>`;
        html += `<span class="hook-col-status"><span class="hook-status-badge">${statusLabel}</span></span>`;
        html += '<span class="hook-col-test">';
        html += `<button class="btn-test btn-test-bus" data-agent="${agent}" data-event="${pt.event_name}">${t('btn_test_eventbus', currentLang)}</button>`;
        html += `<button class="btn-test btn-copy-cmd" data-cmd="${copyCmd.replace(/"/g, '&quot;')}">${t('btn_copy_command', currentLang)}</button>`;
        html += '</span>';
        html += '</div>';
    });

    html += '</div>';
    detailEl.innerHTML = html;

    detailEl.querySelectorAll('.btn-test-bus').forEach(btn => {
        btn.addEventListener('click', async (e) => {
            e.stopPropagation();
            const evt = btn.getAttribute('data-event');
            await testHookPoint(agent, evt);
        });
    });

    detailEl.querySelectorAll('.btn-copy-cmd').forEach(btn => {
        btn.addEventListener('click', async (e) => {
            e.stopPropagation();
            const cmd = btn.getAttribute('data-cmd') || '';
            const origText = btn.textContent;
            try {
                await navigator.clipboard.writeText(cmd);
                btn.textContent = t('btn_copied', currentLang);
                btn.classList.add('copied');
            } catch (err) {
                console.error('Failed to copy command', err);
            }
            setTimeout(() => {
                btn.textContent = origText;
                btn.classList.remove('copied');
            }, 1500);
        });
    });
}

function renderAgentCards() {
    const container = panelContainer;
    if (!container) return;

    container.innerHTML = '';

    AGENTS.forEach(agent => {
        const status = hooksStatusCache[agent];
        const isInjected = status ? status.injected : false;
        const isExpanded = expandedAgents.has(agent);

        const card = document.createElement('div');
        card.className = 'hook-agent-card';
        card.setAttribute('data-agent', agent);

        const header = document.createElement('div');
        header.className = 'hook-agent-header';

        const left = document.createElement('div');
        left.className = 'hook-agent-left';

        const expandIcon = document.createElement('span');
        expandIcon.className = 'hook-expand-icon' + (isExpanded ? ' expanded' : '');
        expandIcon.innerHTML = '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6"/></svg>';

        const nameEl = document.createElement('span');
        nameEl.className = 'hook-agent-name';
        nameEl.textContent = AGENT_DISPLAY[agent] || agent;

        const badge = document.createElement('span');
        badge.id = `badge-${agent}`;
        if (status) {
            badge.className = isInjected ? 'badge badge-injected' : 'badge badge-not-injected';
            badge.textContent = isInjected ? t('badge_injected', currentLang) : t('badge_not_injected', currentLang);
        } else {
            badge.className = 'badge badge-loading';
            badge.textContent = t('badge_loading', currentLang);
        }

        left.appendChild(expandIcon);
        left.appendChild(nameEl);
        left.appendChild(badge);

        const actions = document.createElement('div');
        actions.className = 'hook-agent-actions';

        const btnInject = document.createElement('button');
        btnInject.className = 'secondary-btn btn-sm';
        btnInject.id = `btn-inject-${agent}`;
        btnInject.textContent = t('btn_inject', currentLang);
        btnInject.style.display = isInjected ? 'none' : 'inline-block';

        const btnViewConfig = document.createElement('button');
        btnViewConfig.className = 'secondary-btn btn-sm';
        btnViewConfig.id = `btn-view-config-${agent}`;
        btnViewConfig.textContent = t('btn_view_config', currentLang);
        btnViewConfig.style.display = isInjected ? 'inline-block' : 'none';

        const btnUninstall = document.createElement('button');
        btnUninstall.className = 'danger-btn btn-sm';
        btnUninstall.id = `btn-uninstall-${agent}`;
        btnUninstall.textContent = t('btn_uninstall', currentLang);
        btnUninstall.style.display = isInjected ? 'inline-block' : 'none';

        actions.appendChild(btnInject);
        actions.appendChild(btnViewConfig);
        actions.appendChild(btnUninstall);

        header.appendChild(left);
        header.appendChild(actions);

        header.addEventListener('click', (e) => {
            if (e.target.closest('button')) return;
            toggleAgentExpand(agent);
        });

        card.appendChild(header);

        const detail = document.createElement('div');
        detail.className = 'hook-agent-detail';
        detail.id = `hook-detail-${agent}`;
        if (isExpanded) {
            detail.classList.add('expanded');
            if (hookDetailsCache[agent]) {
                renderHookPointsTable(detail, hookDetailsCache[agent], agent);
            } else {
                detail.innerHTML = `<div class="hook-detail-loading">${t('msg_loading_details', currentLang)}</div>`;
            }
        }
        card.appendChild(detail);

        btnViewConfig.addEventListener('click', async (e) => {
            e.stopPropagation();
            try {
                const content = await invoke('get_config_content', { agent });
                if (hooksStatusCache[agent] && hooksStatusCache[agent].config_path) {
                    document.getElementById('config-viewer-path-text').textContent = hooksStatusCache[agent].config_path;
                    document.getElementById('config-viewer-path-bar').style.display = 'flex';
                } else {
                    document.getElementById('config-viewer-path-text').textContent = '';
                    document.getElementById('config-viewer-path-bar').style.display = 'none';
                }
                configViewerCode.innerHTML = syntaxHighlightJSON(content || "{}");
                configViewerModal.style.display = 'flex';
            } catch (err) {
                console.error(err);
            }
        });

        btnInject.addEventListener('click', async (e) => {
            e.stopPropagation();
            try {
                const diff = await invoke('preview_inject_hook', { agent });
                const { beforeHTML, afterHTML } = renderDiffRows(diff.before, diff.after);
                injectBeforeCode.innerHTML = beforeHTML;
                injectAfterCode.innerHTML = afterHTML;
                if (hooksStatusCache[agent] && hooksStatusCache[agent].config_path) {
                    document.getElementById('inject-path-text').textContent = hooksStatusCache[agent].config_path;
                    document.getElementById('inject-path-bar').style.display = 'flex';
                } else {
                    document.getElementById('inject-path-bar').style.display = 'none';
                }
                currentInjectingAgent = agent;
                hookModal.style.display = 'flex';
            } catch (err) {
                alert("Preview failed: " + err);
            }
        });

        btnUninstall.addEventListener('click', async (e) => {
            e.stopPropagation();
            try {
                const diff = await invoke('preview_uninstall_hook', { agent });
                const { beforeHTML, afterHTML } = renderDiffRows(diff.before, diff.after);
                uninstallBeforeCode.innerHTML = beforeHTML;
                uninstallAfterCode.innerHTML = afterHTML;
                if (hooksStatusCache[agent] && hooksStatusCache[agent].config_path) {
                    document.getElementById('uninstall-path-text').textContent = hooksStatusCache[agent].config_path;
                    document.getElementById('uninstall-path-bar').style.display = 'flex';
                } else {
                    document.getElementById('uninstall-path-bar').style.display = 'none';
                }
                currentUninstallingAgent = agent;
                uninstallModal.style.display = 'flex';
            } catch (err) {
                alert("Preview uninstall failed: " + err);
            }
        });

        container.appendChild(card);
    });
}

async function toggleAgentExpand(agent) {
    const detailEl = document.getElementById(`hook-detail-${agent}`);
    if (!detailEl) return;

    const wasExpanded = expandedAgents.has(agent);

    if (wasExpanded) {
        expandedAgents.delete(agent);
        detailEl.classList.remove('expanded');
        const card = detailEl.closest('.hook-agent-card');
        if (card) {
            const icon = card.querySelector('.hook-expand-icon');
            if (icon) icon.classList.remove('expanded');
        }
    } else {
        expandedAgents.add(agent);
        detailEl.classList.add('expanded');
        const card = detailEl.closest('.hook-agent-card');
        if (card) {
            const icon = card.querySelector('.hook-expand-icon');
            if (icon) icon.classList.add('expanded');
        }

        if (!hookDetailsCache[agent]) {
            try {
                hookDetailsCache[agent] = await invoke('get_hook_details', { agent });
            } catch (e) {
                console.error('Failed to load hook details', e);
                hookDetailsCache[agent] = { hook_points: [] };
            }
        }
        renderHookPointsTable(detailEl, hookDetailsCache[agent], agent);
    }
}

async function testHookPoint(agent, eventName) {
    try {
        const result = await invoke('test_hook_point', { agent, eventName, mode: 'event_bus' });
        if (result.success) {
            showToast(t('msg_test_success', currentLang), 'success');
        } else {
            showToast(t('msg_test_failed', currentLang), 'error');
        }
    } catch (e) {
        console.error('Hook test invoke failed', e);
        showToast(t('msg_test_failed', currentLang), 'error');
    }
}

async function fetchHooksStatus() {
    try {
        const status = await invoke('get_hooks_status');
        if (status) {
            hooksStatusCache = status;
            renderAgentCards();
        }
    } catch (e) {
        console.error("Failed to fetch hooks status", e);
    }
}

// Copy buttons for the modal path bars (bound once after modals are injected).
function bindCopyButtons() {
    document.querySelectorAll('.modal-path-copy').forEach((btn) => {
        btn.addEventListener('click', async () => {
            const targetId = btn.getAttribute('data-target');
            const targetEl = document.getElementById(targetId);
            if (targetEl && targetEl.textContent) {
                try {
                    await navigator.clipboard.writeText(targetEl.textContent);
                    const origText = btn.textContent;
                    btn.textContent = t('btn_copied', currentLang);
                    btn.style.backgroundColor = "rgba(46, 160, 67, 0.3)";
                    btn.style.borderColor = "rgba(46, 160, 67, 0.5)";
                    setTimeout(() => {
                        btn.textContent = origText;
                        btn.style.backgroundColor = "";
                        btn.style.borderColor = "";
                    }, 2000);
                } catch (e) {
                    console.error("Failed to copy:", e);
                }
            }
        });
    });
}

let mounted = false;

/**
 * Mount the full hook management panel into `container`.
 * @param {HTMLElement} container Element to render the agent cards into.
 * @param {string} [lang] Optional language override; otherwise loaded from config.
 */
export async function mountHookPanel(container, lang) {
    if (!container) return;
    panelContainer = container;

    if (!mounted) {
        if (!document.getElementById('hook-modal')) {
            document.body.insertAdjacentHTML('beforeend', HOOK_MODALS_HTML);
        }
        hookModal = document.getElementById('hook-modal');
        const btnModalCancel = document.getElementById('btn-modal-cancel');
        btnModalConfirm = document.getElementById('btn-modal-confirm');
        injectBeforeCode = document.getElementById('inject-before-code');
        injectAfterCode = document.getElementById('inject-after-code');
        uninstallModal = document.getElementById('uninstall-modal');
        const btnUninstallCancel = document.getElementById('btn-uninstall-cancel');
        btnUninstallConfirm = document.getElementById('btn-uninstall-confirm');
        uninstallBeforeCode = document.getElementById('uninstall-before-code');
        uninstallAfterCode = document.getElementById('uninstall-after-code');
        configViewerModal = document.getElementById('config-viewer-modal');
        const btnConfigViewerClose = document.getElementById('btn-config-viewer-close');
        configViewerCode = document.getElementById('config-viewer-code');

        bindCopyButtons();

        btnConfigViewerClose.addEventListener('click', () => {
            configViewerModal.style.display = 'none';
        });

        btnUninstallCancel.addEventListener('click', () => {
            uninstallModal.style.display = 'none';
            currentUninstallingAgent = null;
        });

        btnUninstallConfirm.addEventListener('click', async () => {
            if (!currentUninstallingAgent) return;
            try {
                btnUninstallConfirm.disabled = true;
                await invoke('uninstall_hook', { agent: currentUninstallingAgent });
                uninstallModal.style.display = 'none';
                await fetchHooksStatus();
            } catch (e) {
                alert("Uninstall failed: " + e);
            } finally {
                btnUninstallConfirm.disabled = false;
                currentUninstallingAgent = null;
            }
        });

        btnModalCancel.addEventListener('click', () => {
            hookModal.style.display = 'none';
            currentInjectingAgent = null;
        });

        btnModalConfirm.addEventListener('click', async () => {
            if (!currentInjectingAgent) return;
            try {
                btnModalConfirm.disabled = true;
                await invoke('inject_hook', { agent: currentInjectingAgent });
                hookModal.style.display = 'none';
                await fetchHooksStatus();
            } catch (e) {
                alert("Inject failed: " + e);
            } finally {
                btnModalConfirm.disabled = false;
                currentInjectingAgent = null;
            }
        });

        mounted = true;
    }

    // Load config for the event-status map and default language.
    try {
        currentConfig = await invoke('get_config');
        if (currentConfig && currentConfig.general && currentConfig.general.language) {
            currentLang = currentConfig.general.language;
        }
    } catch (e) {
        console.error('Failed to load config', e);
    }
    if (lang) currentLang = lang;
    applyTranslations(currentLang);

    await fetchHooksStatus();
}
