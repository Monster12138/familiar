import { applyTranslations, t } from './i18n.js';
import { mountHookPanel } from './hook-panel.js';

const { invoke } = window.__TAURI__.core;
const { getCurrentWebviewWindow } = window.__TAURI__.webviewWindow;

let lang = 'zh-CN';
let currentConfig = null;
let hooksMounted = false;

function showToast(message, type, durationMs = 2400) {
    let toast = document.querySelector('.app-toast');
    if (!toast) {
        toast = document.createElement('div');
        toast.className = 'app-toast';
        document.body.appendChild(toast);
    }
    toast.textContent = message;
    toast.className = `app-toast${type ? ` ${type}` : ''} show`;
    window.clearTimeout(showToast.timeoutId);
    showToast.timeoutId = window.setTimeout(() => toast.classList.remove('show'), durationMs);
}

function boundedInteger(input, fallback, min, max) {
    const value = Number.parseInt(input.value, 10);
    if (!Number.isFinite(value)) return fallback;
    return Math.min(max, Math.max(min, value));
}

function showView(viewId) {
    ['onboard-intro', 'onboard-hook-view', 'onboard-remote-view'].forEach((id) => {
        const view = document.getElementById(id);
        if (view) view.style.display = id === viewId ? 'block' : 'none';
    });
}

async function saveConfig() {
    await invoke('save_config', { config: currentConfig });
}

// Mark onboarding complete so the welcome page does not reappear.
async function completeOnboarding() {
    try {
        await invoke('complete_onboarding');
    } catch (e) {
        console.error('Failed to complete onboarding', e);
    }
}

// Step 2: swap the introduction for the shared hook management panel
// mounted in the same window.
async function showHooks() {
    currentConfig.runtime = currentConfig.runtime || {};
    currentConfig.runtime.mode = 'local';
    await saveConfig();
    showView('onboard-hook-view');
    if (!hooksMounted) {
        await mountHookPanel(document.getElementById('onboard-hook-panel'));
        hooksMounted = true;
    }
}

async function showRemote() {
    showView('onboard-remote-view');
    try {
        const win = getCurrentWebviewWindow();
        if (win) await win.setFocus();
    } catch (error) {
        // The window is already interactive on some macOS WebView versions,
        // where an explicit focus request may be rejected. Input focus below
        // remains sufficient and this must not block remote setup.
        console.debug('Onboarding window focus request was not needed', error);
    }
    window.requestAnimationFrame(() => {
        document.getElementById('onboard-remote-endpoint')?.focus({ preventScroll: true });
    });
}

async function proceedFromMode() {
    const mode = document.querySelector('input[name="onboard-mode"]:checked')?.value || 'local';
    try {
        if (mode === 'remote') await showRemote();
        else await showHooks();
    } catch (error) {
        console.error('Failed to select onboarding mode', error);
        showToast(t('msg_auto_save_error', lang), 'error');
    }
}

async function saveRemoteAndFinish() {
    const endpointInput = document.getElementById('onboard-remote-endpoint');
    const endpoint = endpointInput.value.trim();
    if (!endpoint) {
        endpointInput.focus();
        showToast(t('onboard_remote_endpoint_required', lang), 'error');
        return;
    }

    const tokenInput = document.getElementById('onboard-remote-token');
    const initialInput = document.getElementById('onboard-remote-reconnect-initial');
    const maxInput = document.getElementById('onboard-remote-reconnect-max');
    const reconnectInitial = boundedInteger(initialInput, 1, 1, 3600);

    currentConfig.runtime = currentConfig.runtime || {};
    currentConfig.remote = currentConfig.remote || {};
    currentConfig.runtime.mode = 'remote';
    currentConfig.remote.endpoint = endpoint;
    currentConfig.remote.path = document.getElementById('onboard-remote-path').value.trim() || '/api/v1/state-stream';
    currentConfig.remote.tls = document.getElementById('onboard-remote-tls').checked;
    currentConfig.remote.connect_timeout_secs = boundedInteger(
        document.getElementById('onboard-remote-connect-timeout'), 10, 1, 600,
    );
    currentConfig.remote.reconnect_initial_secs = reconnectInitial;
    currentConfig.remote.reconnect_max_secs = boundedInteger(maxInput, 30, reconnectInitial, 86400);

    try {
        const token = tokenInput.value.trim();
        if (token) {
            currentConfig.remote.token_file = await invoke('save_remote_token', { token });
            tokenInput.value = '';
        }
        await saveConfig();
        await finishOnboarding();
    } catch (error) {
        console.error('Failed to save remote onboarding settings', error);
        showToast(t('msg_auto_save_error', lang), 'error');
    }
}

// Finish setup after the hooks step: complete onboarding and close.
async function finishOnboarding() {
    await completeOnboarding();
    const win = getCurrentWebviewWindow();
    if (win) win.close();
}

document.addEventListener('DOMContentLoaded', async () => {
    try {
        currentConfig = await invoke('get_config');
        if (currentConfig?.general?.language) {
            lang = currentConfig.general.language;
        }
    } catch (e) {
        console.error('Failed to load config', e);
    }
    applyTranslations(lang);

    const remote = currentConfig?.remote || {};
    document.getElementById('onboard-remote-endpoint').value = remote.endpoint || '';
    document.getElementById('onboard-remote-path').value = remote.path || '/api/v1/state-stream';
    document.getElementById('onboard-remote-tls').checked = remote.tls === true;
    document.getElementById('onboard-remote-connect-timeout').value = remote.connect_timeout_secs ?? 10;
    document.getElementById('onboard-remote-reconnect-initial').value = remote.reconnect_initial_secs ?? 1;
    document.getElementById('onboard-remote-reconnect-max').value = remote.reconnect_max_secs ?? 30;

    const nextBtn = document.getElementById('btn-onboard-next');
    const skipBtn = document.getElementById('btn-onboard-skip');
    const localDoneBtn = document.getElementById('btn-onboard-local-done');
    const remoteDoneBtn = document.getElementById('btn-onboard-remote-done');
    if (nextBtn) nextBtn.addEventListener('click', proceedFromMode);
    if (skipBtn) skipBtn.addEventListener('click', finishOnboarding);
    if (localDoneBtn) localDoneBtn.addEventListener('click', finishOnboarding);
    if (remoteDoneBtn) remoteDoneBtn.addEventListener('click', saveRemoteAndFinish);
    document.querySelectorAll('.btn-onboard-back').forEach((button) => {
        button.addEventListener('click', () => showView('onboard-intro'));
    });
});
