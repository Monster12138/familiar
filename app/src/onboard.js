import { applyTranslations, t } from './i18n.js';

const { invoke } = window.__TAURI__.core;
const { getCurrentWebviewWindow } = window.__TAURI__.webviewWindow;

let lang = 'zh-CN';

// Render a read-only per-agent injection status readout. Full management
// (inject / uninstall / config-path / details / test) lives in the dedicated
// hook manager window, opened via the button below.
async function renderHooksSummary() {
    const container = document.getElementById('onboard-hooks-summary');
    if (!container) return;
    try {
        const status = await invoke('get_hooks_status');
        const names = {
            'antigravity': 'Antigravity',
            'claude-code': 'Claude Code',
            'codex': 'Codex',
            'qoder': 'Qoder',
        };
        container.innerHTML = '';
        ['antigravity', 'claude-code', 'codex', 'qoder'].forEach((agent) => {
            const st = status[agent];
            const injected = st ? st.injected : false;
            const badgeClass = st
                ? (injected ? 'badge badge-injected' : 'badge badge-not-injected')
                : 'badge badge-loading';
            const badgeText = st
                ? (injected ? t('badge_injected', lang) : t('badge_not_injected', lang))
                : t('badge_loading', lang);
            const item = document.createElement('span');
            item.className = 'hook-summary-item';
            item.innerHTML = `<span class="hook-summary-agent">${names[agent] || agent}</span> <span class="${badgeClass}">${badgeText}</span>`;
            container.appendChild(item);
        });
    } catch (e) {
        console.error('Failed to load hooks summary', e);
    }
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

    const btnOpenHooks = document.getElementById('btn-onboard-open-hooks');
    if (btnOpenHooks) {
        btnOpenHooks.addEventListener('click', () => {
            invoke('open_hook_manager_window');
        });
    }

    await renderHooksSummary();
});
