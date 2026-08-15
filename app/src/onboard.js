import { applyTranslations, t } from './i18n.js';
import { renderDiffRows } from './diff.js';

const { invoke } = window.__TAURI__.core;
const { getCurrentWebviewWindow } = window.__TAURI__.webviewWindow;

const AGENT_DISPLAY = {
    'antigravity': 'Antigravity',
    'claude-code': 'Claude Code',
    'codex': 'Codex',
    'qoder': 'Qoder',
};

const AGENTS = ['antigravity', 'claude-code', 'codex', 'qoder'];

let lang = 'zh-CN';
let currentOnboardInjectAgent = null;
let onboardHookModal = null;

// Same global toast used by the settings panel, replicated here so the
// onboard page has its own transient-notification channel.
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

// Render one card per supported agent: name, injected/not-injected badge,
// and a one-click inject button (hidden once injected). Injections write
// directly to the agent's own config file; familiar backs it up first.
async function loadAgentCards() {
    const container = document.getElementById('onboard-agent-list');
    if (!container) return;
    container.innerHTML = '';

    let statusCache = {};
    try {
        statusCache = await invoke('get_hooks_status');
    } catch (e) {
        console.error('Failed to load hooks status', e);
    }

    AGENTS.forEach((agent) => {
        const status = statusCache[agent];
        const isInjected = status ? status.injected : false;

        const card = document.createElement('div');
        card.className = 'hook-agent-card onboard-agent-card';
        card.setAttribute('data-agent', agent);

        const left = document.createElement('div');
        left.className = 'onboard-agent-left';

        const name = document.createElement('span');
        name.className = 'onboard-agent-name';
        name.textContent = AGENT_DISPLAY[agent] || agent;

        const badge = document.createElement('span');
        badge.id = `onboard-badge-${agent}`;
        if (status) {
            badge.className = isInjected ? 'badge badge-injected' : 'badge badge-not-injected';
            badge.textContent = isInjected
                ? t('badge_injected', lang)
                : t('badge_not_injected', lang);
        } else {
            badge.className = 'badge badge-loading';
            badge.textContent = t('badge_loading', lang);
        }

        left.appendChild(name);
        left.appendChild(badge);

        const btn = document.createElement('button');
        btn.className = 'secondary-btn btn-sm';
        btn.id = `onboard-inject-${agent}`;
        btn.textContent = t('btn_inject', lang);
        btn.style.display = isInjected ? 'none' : 'inline-block';

        // Show the before/after diff in the preview modal first; the actual
        // injection only happens after the user confirms.
        btn.addEventListener('click', async () => {
            try {
                const diff = await invoke('preview_inject_hook', { agent });
                const { beforeHTML, afterHTML } = renderDiffRows(diff.before, diff.after);
                document.getElementById('onboard-inject-before').innerHTML = beforeHTML;
                document.getElementById('onboard-inject-after').innerHTML = afterHTML;
                currentOnboardInjectAgent = agent;
                onboardHookModal.style.display = 'flex';
            } catch (e) {
                console.error('Preview failed', e);
                showToast(
                    t('msg_onboard_inject_failed', lang) + ' (' + (AGENT_DISPLAY[agent] || agent) + ')',
                    'error'
                );
            }
        });

        card.appendChild(left);
        card.appendChild(btn);
        container.appendChild(card);
    });
}

// Persist the "onboarded" flag (so the welcome page is not shown again) and
// close the window. Reached via both the start and skip buttons.
async function completeOnboarding() {
    try {
        await invoke('complete_onboarding');
    } catch (e) {
        console.error('Failed to complete onboarding', e);
    }
    const win = getCurrentWebviewWindow();
    if (win) win.close();
}

document.addEventListener('DOMContentLoaded', async () => {
    try {
        const config = await invoke('get_config');
        if (config && config.general && config.general.language) {
            lang = config.general.language;
        }
    } catch (e) {
        console.error('Failed to load config', e);
    }
    applyTranslations(lang);

    const startBtn = document.getElementById('btn-onboard-start');
    const skipBtn = document.getElementById('btn-onboard-skip');
    if (startBtn) startBtn.addEventListener('click', completeOnboarding);
    if (skipBtn) skipBtn.addEventListener('click', completeOnboarding);

    // Hook inject preview modal: confirm performs the injection, cancel closes.
    onboardHookModal = document.getElementById('onboard-hook-modal');
    const btnInjectCancel = document.getElementById('btn-onboard-inject-cancel');
    const btnInjectConfirm = document.getElementById('btn-onboard-inject-confirm');

    if (btnInjectCancel) {
        btnInjectCancel.addEventListener('click', () => {
            onboardHookModal.style.display = 'none';
            currentOnboardInjectAgent = null;
        });
    }

    if (btnInjectConfirm) {
        btnInjectConfirm.addEventListener('click', async () => {
            if (!currentOnboardInjectAgent) return;
            const agent = currentOnboardInjectAgent;
            try {
                btnInjectConfirm.disabled = true;
                await invoke('inject_hook', { agent });
                onboardHookModal.style.display = 'none';
                currentOnboardInjectAgent = null;
                showToast(
                    t('msg_onboard_inject_success', lang) + ' (' + (AGENT_DISPLAY[agent] || agent) + ')',
                    'success'
                );
                await loadAgentCards();
            } catch (e) {
                console.error('Inject failed', e);
                showToast(
                    t('msg_onboard_inject_failed', lang) + ' (' + (AGENT_DISPLAY[agent] || agent) + ')',
                    'error'
                );
            } finally {
                btnInjectConfirm.disabled = false;
            }
        });
    }

    await loadAgentCards();
});
