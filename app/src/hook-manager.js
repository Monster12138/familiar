// Standalone hook manager window. The full agent hook management UI
// (inject / uninstall / view-config / config path / hook-point details /
// test) extracted from the settings panel so both the settings page and the
// onboarding page can open the same window. Reuses diff.js for line-based
// config diffs and i18n.js for translations.

import { applyTranslations, t } from './i18n.js';
import { renderDiffRows, syntaxHighlightJSON } from './diff.js';

const { invoke } = window.__TAURI__.core;

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
    'qoder': 'Qoder',
};

const AGENTS = ['antigravity', 'claude-code', 'codex', 'qoder'];

let lang = 'zh-CN';
let currentConfig = {};
let currentInjectingAgent = null;
let currentUninstallingAgent = null;
let hooksStatusCache = {};
let hookDetailsCache = {};      // agent -> AgentHookDetail (lazy)
let expandedAgents = new Set(); // track which agent cards are expanded

// Modal DOM refs, assigned in DOMContentLoaded; the card handlers in
// renderAgentCards reference them.
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

// Copy buttons for the modal path bars.
function bindCopyButtons() {
    document.querySelectorAll('.modal-path-copy').forEach((btn) => {
        btn.addEventListener('click', async () => {
            const targetId = btn.getAttribute('data-target');
            const targetEl = document.getElementById(targetId);
            if (targetEl && targetEl.textContent) {
                try {
                    await navigator.clipboard.writeText(targetEl.textContent);
                    const origText = btn.textContent;
                    btn.textContent = t('btn_copied', lang);
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
        detailEl.innerHTML = `<div class="hook-detail-empty">${t('lbl_no_hook_points', lang)}</div>`;
        return;
    }

    let html = '<div class="hook-points-table">';
    html += '<div class="hook-points-header">';
    html += `<span class="hook-col-event">${t('lbl_hook_event', lang)}</span>`;
    html += `<span class="hook-col-status">${t('lbl_hook_status', lang)}</span>`;
    html += `<span class="hook-col-test"></span>`;
    html += '</div>';

    points.forEach(pt => {
        const matcherLabel = pt.matcher ? ` (matcher: ${pt.matcher})` : '';
        const statusKey = hookEventStatus(pt.event_name, agent);
        const statusLabel = statusKey
            ? t('status_' + statusKey.replace(/-/g, '_'), lang)
            : t('status_default_noop', lang);
        // The command is only kept for copying (data-cmd), not displayed.
        const copyCmd = pt.test_command || pt.command;
        html += '<div class="hook-point-row">';
        html += `<span class="hook-col-event"><code>${pt.event_name}</code>${matcherLabel}</span>`;
        html += `<span class="hook-col-status"><span class="hook-status-badge">${statusLabel}</span></span>`;
        html += '<span class="hook-col-test">';
        html += `<button class="btn-test btn-test-bus" data-agent="${agent}" data-event="${pt.event_name}">${t('btn_test_eventbus', lang)}</button>`;
        html += `<button class="btn-test btn-copy-cmd" data-cmd="${copyCmd.replace(/"/g, '&quot;')}">${t('btn_copy_command', lang)}</button>`;
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
                btn.textContent = t('btn_copied', lang);
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
    const container = document.getElementById('hook-status-list');
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
            badge.textContent = isInjected ? t('badge_injected', lang) : t('badge_not_injected', lang);
        } else {
            badge.className = 'badge badge-loading';
            badge.textContent = t('badge_loading', lang);
        }

        left.appendChild(expandIcon);
        left.appendChild(nameEl);
        left.appendChild(badge);

        const actions = document.createElement('div');
        actions.className = 'hook-agent-actions';

        const btnInject = document.createElement('button');
        btnInject.className = 'secondary-btn btn-sm';
        btnInject.id = `btn-inject-${agent}`;
        btnInject.textContent = t('btn_inject', lang);
        btnInject.style.display = isInjected ? 'none' : 'inline-block';

        const btnViewConfig = document.createElement('button');
        btnViewConfig.className = 'secondary-btn btn-sm';
        btnViewConfig.id = `btn-view-config-${agent}`;
        btnViewConfig.textContent = t('btn_view_config', lang);
        btnViewConfig.style.display = isInjected ? 'inline-block' : 'none';

        const btnUninstall = document.createElement('button');
        btnUninstall.className = 'danger-btn btn-sm';
        btnUninstall.id = `btn-uninstall-${agent}`;
        btnUninstall.textContent = t('btn_uninstall', lang);
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
                detail.innerHTML = `<div class="hook-detail-loading">${t('msg_loading_details', lang)}</div>`;
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
            showToast(t('msg_test_success', lang), 'success');
        } else {
            showToast(t('msg_test_failed', lang), 'error');
        }
    } catch (e) {
        console.error('Hook test invoke failed', e);
        showToast(t('msg_test_failed', lang), 'error');
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

document.addEventListener('DOMContentLoaded', async () => {
    try {
        currentConfig = await invoke('get_config');
        if (currentConfig && currentConfig.general && currentConfig.general.language) {
            lang = currentConfig.general.language;
        }
    } catch (e) {
        console.error('Failed to load config', e);
    }
    applyTranslations(lang);

    // DOM refs for the three modals.
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

    await fetchHooksStatus();
});
