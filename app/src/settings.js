import { applyTranslations, t } from './i18n.js';
import { mountHookPanel } from './hook-panel.js';

const { invoke } = window.__TAURI__.core;
const { getCurrentWebviewWindow } = window.__TAURI__.webviewWindow;

// AgentEventType kinds exposed in the event-status mapping table (excluding
// the non-mappable AgentStopped). Order matches the backend kind() strings.
const EVENT_KINDS = [
    'AgentStarted', 'Thinking', 'Processing', 'ReadingFile', 'WritingFile',
    'RunningCommand', 'SearchingCode', 'BrowsingWeb', 'TaskCompleted',
    'TaskFailed', 'WaitingForInput', 'SubagentStarted', 'SubagentStopped'
];
// AgentStatus values in config TOML (kebab-case).
const EVENT_STATUS_OPTIONS = ['idle', 'thinking', 'working', 'pending', 'completed', 'failed'];

// Built-in (fallback) status per event kind, mirroring StateMachine::apply_event.
// `null` means the event is a no-op by default (keeps the agent's current status).
const EVENT_DEFAULT_STATUS = {
    'AgentStarted': 'idle',
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

document.addEventListener('DOMContentLoaded', async () => {
    // Disable default context menu (developer tools / inspect element)
    document.addEventListener('contextmenu', (e) => {
        e.preventDefault();
    });

    // Nav elements
    const menuItems = document.querySelectorAll('.menu-item');
    const contentTitle = document.getElementById('content-title');
    const contentScroll = document.getElementById('content-scroll');

    // Form elements
    const elLanguage = document.getElementById('setting-language');
    const elApiPort = document.getElementById('setting-api-port');
    const elRuntimeMode = document.getElementById('setting-runtime-mode');
    const elRemoteEndpoint = document.getElementById('setting-remote-endpoint');
    const elRemotePath = document.getElementById('setting-remote-path');
    const elRemoteTls = document.getElementById('setting-remote-tls');
    const elRemoteToken = document.getElementById('setting-remote-token');
    const elRemoteConnectTimeout = document.getElementById('setting-remote-connect-timeout');
    const elRemoteReconnectInitial = document.getElementById('setting-remote-reconnect-initial');
    const elRemoteReconnectMax = document.getElementById('setting-remote-reconnect-max');
    const remoteSettingItems = document.querySelectorAll('.runtime-remote-setting');
    
    const elPetScale = document.getElementById('setting-pet-scale');
    const valPetScale = document.getElementById('val-pet-scale');
    const elPetAlwaysTop = document.getElementById('setting-pet-always-top');
    const elPetAllDesktops = document.getElementById('setting-pet-all-desktops');
    const elPetClickThrough = document.getElementById('setting-pet-click-through');
    const elPetHideOnHover = document.getElementById('setting-pet-hide-on-hover');
    const elPetWindowFrame = document.getElementById('setting-pet-window-frame');
    const elPetOpacity = document.getElementById('setting-pet-opacity');
    const valPetOpacity = document.getElementById('val-pet-opacity');
    const elPetHoverOpacity = document.getElementById('setting-pet-hover-opacity');
    const valPetHoverOpacity = document.getElementById('val-pet-hover-opacity');
    const elPetSnapCorner = document.getElementById('setting-pet-snap-corner');
    const elShowBubble = document.getElementById('setting-show-bubble');
    const elShowPet = document.getElementById('setting-show-pet');
    const elShowStats = document.getElementById('setting-show-stats');
    const elDashboardStyle = document.getElementById('setting-dashboard-style');
    const elDashboardPosition = document.getElementById('setting-dashboard-position');
    const elDashboardLayout = document.getElementById('setting-dashboard-layout');
    const elDashboardAlignment = document.getElementById('setting-dashboard-alignment');

    const elUdsPath = document.getElementById('setting-uds-path');
    const elTcpPort = document.getElementById('setting-tcp-port');

    const elCleanupBackups = document.getElementById('setting-cleanup-backups');
    const elCleanupLogs = document.getElementById('setting-cleanup-logs');
    const elCleanupAgeDays = document.getElementById('setting-cleanup-age-days');
    const btnCleanupRun = document.getElementById('btn-cleanup-run');

    const elUpdateStartup = document.getElementById('setting-update-startup');
    const elUpdateInterval = document.getElementById('setting-update-interval');
    const elUpdateCurrentVersion = document.getElementById('setting-update-current-version');
    const btnCheckUpdate = document.getElementById('btn-check-update');
    const aboutVersion = document.getElementById('about-version');

    // Hide settings that only apply on some platforms: UDS has no Unix
    // domain sockets on Windows, and "follow desktop switching" only has
    // an effect on macOS (Spaces / full-screen behavior).
    const hideSettingItem = (input) => {
        if (!input) return;
        const item = input.closest('.setting-item');
        if (!item) return;
        item.style.display = 'none';
        const next = item.nextElementSibling;
        const prev = item.previousElementSibling;
        if (next && next.classList.contains('divider')) next.style.display = 'none';
        else if (prev && prev.classList.contains('divider')) prev.style.display = 'none';
    };

    try {
        const platform = await invoke('get_platform');
        if (platform === 'windows') hideSettingItem(elUdsPath);
        if (platform !== 'darwin') hideSettingItem(elPetAllDesktops);
    } catch (e) {
        console.error('Failed to detect platform:', e);
    }
    const elCelebrationSecs = document.getElementById('setting-celebration-secs');
    const valCelebrationSecs = document.getElementById('val-celebration-secs');
    const elSleepTimeoutSecs = document.getElementById('setting-sleep-timeout-secs');
    const valSleepTimeoutSecs = document.getElementById('val-sleep-timeout-secs');

    const saveBtn = document.getElementById('save-btn');
    const sessionListContainer = document.getElementById('session-list-container');

    // Sprite pack UI elements
    const convertFileSrc = window.__TAURI__.core.convertFileSrc || ((p) => p);
    const spritePackGrid = document.getElementById('sprite-pack-grid');
    const btnOpenSpriteDir = document.getElementById('btn-open-sprite-dir');
    const btnImportPack = document.getElementById('btn-import-pack');
    const fileImportPack = document.getElementById('file-import-pack');

    // Preview modal elements
    const previewModalOverlay = document.getElementById('preview-modal-overlay');
    const previewModalSubInfo = document.getElementById('preview-modal-sub-info');
    const previewModalStateGrid = document.getElementById('preview-modal-state-grid');
    const btnClosePreviewModal = document.getElementById('btn-close-preview-modal');

    let currentConfig = {};
    let autoSaveTimer = null;
    let currentActiveAgents = [];

    function updateRemoteSettingsVisibility() {
        const isRemote = elRuntimeMode?.value === 'remote';
        remoteSettingItems.forEach((item) => {
            item.classList.toggle('runtime-setting-disabled', !isRemote);
            item.querySelectorAll('input, select, textarea').forEach((input) => {
                input.disabled = !isRemote;
            });
        });
    }

    function updateRemoteTokenPlaceholder() {
        if (!elRemoteToken) return;
        const key = currentConfig.remote?.token_file
            ? 'placeholder_remote_token_saved'
            : 'placeholder_remote_token';
        elRemoteToken.placeholder = t(key, elLanguage?.value || 'zh-CN');
    }

    // Session card action icons (Feather-style, stroke=currentColor)
    const ICON_EYE = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>';
    const ICON_EYE_OFF = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>';
    const ICON_TRASH = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>';

    function getSourceBadgeHtml(source) {
        const srcStr = typeof source === 'string' ? source : (source?.Custom || 'Agent');
        if (srcStr === 'Codex') {
            return `<span class="session-source-badge" style="background:#10a37f;">Codex</span>`;
        } else if (srcStr === 'ClaudeCode' || srcStr === 'Claude') {
            return `<span class="session-source-badge" style="background:#d97706;">Claude</span>`;
        } else if (srcStr === 'Antigravity' || srcStr === 'Agy') {
            return `<span class="session-source-badge" style="background:#4f46e5;">AGY</span>`;
        } else if (srcStr === 'Qoder') {
            return `<span class="session-source-badge" style="background:#0284c7;">Qoder</span>`;
        } else if (srcStr === 'DeepSeekHarness') {
            return `<span class="session-source-badge" style="background:#4d6bfe;">DSH</span>`;
        }
        return `<span class="session-source-badge" style="background:#6b7280;">${srcStr}</span>`;
    }

    function renderSessionList(agents) {
        if (!sessionListContainer) return;
        currentActiveAgents = agents || [];
        const hiddenSessions = currentConfig.sessions?.hidden_sessions || [];

        if (!currentActiveAgents || currentActiveAgents.length === 0) {
            sessionListContainer.innerHTML = `
                <div class="session-empty-state" data-i18n="lbl_no_active_sessions">
                    ${t('lbl_no_active_sessions', elLanguage ? elLanguage.value : 'zh-CN')}
                </div>
            `;
            return;
        }

        // Clean up empty state if present
        const emptyState = sessionListContainer.querySelector('.session-empty-state');
        if (emptyState) {
            sessionListContainer.removeChild(emptyState);
        }

        const activeIds = new Set(currentActiveAgents.map(a => a.id));

        // Remove cards for agents that are no longer active
        const existingCards = sessionListContainer.querySelectorAll('.session-card-item');
        existingCards.forEach(card => {
            const cardId = card.getAttribute('data-session-id');
            if (cardId && !activeIds.has(cardId)) {
                card.remove();
            }
        });

        // Add or update cards for each active agent
        currentActiveAgents.forEach(agent => {
            const isVisible = !hiddenSessions.includes(agent.id);
            let card = sessionListContainer.querySelector(`[data-session-id="${CSS.escape(agent.id)}"]`);

            const statusLabelText = isVisible
                ? agent.status
                : `${agent.status} (${t('lbl_session_hidden', elLanguage ? elLanguage.value : 'zh-CN')})`;

            const instructionText = agent.user_instruction || agent.current_activity || t('status_waiting', elLanguage ? elLanguage.value : 'zh-CN');

            if (!card) {
                // Create new card
                card = document.createElement('div');
                card.className = 'session-card-item';
                card.setAttribute('data-session-id', agent.id);
                card.style.opacity = isVisible ? '1.0' : '0.75';

                const info = document.createElement('div');
                info.className = 'session-card-info';

                const headerRow = document.createElement('div');
                headerRow.className = 'session-header-row';
                headerRow.innerHTML = `
                    ${getSourceBadgeHtml(agent.source)}
                    <span class="session-id-text" title="${agent.id}">${agent.id.slice(0, 16)}...</span>
                    <span class="session-status-badge" style="${!isVisible ? 'color: var(--text-muted);' : ''}">${statusLabelText}</span>
                `;

                const instructionEl = document.createElement('div');
                instructionEl.className = 'session-instruction-text';
                instructionEl.textContent = instructionText;

                info.appendChild(headerRow);
                info.appendChild(instructionEl);

                // Action buttons: hide (eye toggle) and delete (trash)
                const actions = document.createElement('div');
                actions.className = 'session-actions';

                const btnHide = document.createElement('button');
                btnHide.type = 'button';
                btnHide.className = 'session-icon-btn btn-session-hide' + (isVisible ? '' : ' active');
                btnHide.title = isVisible
                    ? t('lbl_session_hide', elLanguage ? elLanguage.value : 'zh-CN')
                    : t('lbl_session_show', elLanguage ? elLanguage.value : 'zh-CN');
                btnHide.innerHTML = isVisible ? ICON_EYE : ICON_EYE_OFF;

                btnHide.addEventListener('click', () => {
                    if (!currentConfig.sessions) currentConfig.sessions = { hidden_sessions: [] };
                    if (!Array.isArray(currentConfig.sessions.hidden_sessions)) {
                        currentConfig.sessions.hidden_sessions = [];
                    }

                    const willHide = !currentConfig.sessions.hidden_sessions.includes(agent.id);
                    const badge = card.querySelector('.session-status-badge');
                    if (willHide) {
                        currentConfig.sessions.hidden_sessions.push(agent.id);
                        card.style.opacity = '0.75';
                        btnHide.classList.add('active');
                        btnHide.title = t('lbl_session_show', elLanguage ? elLanguage.value : 'zh-CN');
                        btnHide.innerHTML = ICON_EYE_OFF;
                        if (badge) {
                            badge.textContent = `${agent.status} (${t('lbl_session_hidden', elLanguage ? elLanguage.value : 'zh-CN')})`;
                            badge.style.color = 'var(--text-muted)';
                        }
                    } else {
                        currentConfig.sessions.hidden_sessions = currentConfig.sessions.hidden_sessions.filter(id => id !== agent.id);
                        card.style.opacity = '1.0';
                        btnHide.classList.remove('active');
                        btnHide.title = t('lbl_session_hide', elLanguage ? elLanguage.value : 'zh-CN');
                        btnHide.innerHTML = ICON_EYE;
                        if (badge) {
                            badge.textContent = agent.status;
                            badge.style.color = '';
                        }
                    }
                    scheduleAutoSave();
                });

                const btnDelete = document.createElement('button');
                btnDelete.type = 'button';
                btnDelete.className = 'session-icon-btn danger';
                btnDelete.title = t('lbl_session_delete', elLanguage ? elLanguage.value : 'zh-CN');
                btnDelete.innerHTML = ICON_TRASH;

                btnDelete.addEventListener('click', async () => {
                    const lang = elLanguage ? elLanguage.value : 'zh-CN';
                    try {
                        const removed = await invoke('delete_session', { agentId: agent.id });
                        if (removed) {
                            card.remove();
                            currentActiveAgents = currentActiveAgents.filter(a => a.id !== agent.id);
                            // Forget any hide state so a future session is shown by default
                            if (Array.isArray(currentConfig.sessions?.hidden_sessions)) {
                                currentConfig.sessions.hidden_sessions = currentConfig.sessions.hidden_sessions.filter(id => id !== agent.id);
                            }
                            showToast(t('msg_session_deleted', lang), 'success');
                        } else {
                            showToast(t('msg_session_not_found', lang), 'error');
                        }
                    } catch (e) {
                        console.error('Failed to delete session', e);
                        showToast(t('msg_session_delete_failed', lang), 'error');
                    }
                });

                actions.appendChild(btnHide);
                actions.appendChild(btnDelete);

                card.appendChild(info);
                card.appendChild(actions);
                sessionListContainer.appendChild(card);
            } else {
                // Update existing card in place without destroying DOM elements
                card.style.opacity = isVisible ? '1.0' : '0.75';

                const badge = card.querySelector('.session-status-badge');
                if (badge && badge.textContent !== statusLabelText) {
                    badge.textContent = statusLabelText;
                    badge.style.color = !isVisible ? 'var(--text-muted)' : '';
                }

                const instructionEl = card.querySelector('.session-instruction-text');
                if (instructionEl && instructionEl.textContent !== instructionText) {
                    instructionEl.textContent = instructionText;
                }

                // Sync hide-button state with current visibility
                const btnHide = card.querySelector('.btn-session-hide');
                if (btnHide) {
                    const hidden = !isVisible;
                    btnHide.classList.toggle('active', hidden);
                    btnHide.title = hidden
                        ? t('lbl_session_show', elLanguage ? elLanguage.value : 'zh-CN')
                        : t('lbl_session_hide', elLanguage ? elLanguage.value : 'zh-CN');
                    btnHide.innerHTML = hidden ? ICON_EYE_OFF : ICON_EYE;
                }
            }
        });
    }

    // --- Tab Navigation Logic ---
    menuItems.forEach(item => {
        item.addEventListener('click', (e) => {
            e.preventDefault();
            // Update active state
            menuItems.forEach(nav => nav.classList.remove('active'));
            item.classList.add('active');
            
            // Update Title
            contentTitle.textContent = item.textContent.trim();

            // Scroll to target section
            const targetId = item.getAttribute('data-target');
            if (targetId) {
                const section = document.getElementById(targetId);
                if (section) {
                    section.scrollIntoView({ behavior: 'smooth', block: 'start' });
                }
            }
        });
    });

    // Sync active tab on scroll
    contentScroll.addEventListener('scroll', () => {
        const sections = document.querySelectorAll('.settings-group');
        let currentSection = sections[0];
        
        sections.forEach(section => {
            const sectionTop = section.offsetTop - contentScroll.offsetTop;
            if (contentScroll.scrollTop >= sectionTop - 100) {
                currentSection = section;
            }
        });

        const targetId = currentSection.id;
        menuItems.forEach(item => {
            item.classList.remove('active');
            if (item.getAttribute('data-target') === targetId) {
                item.classList.add('active');
                contentTitle.textContent = item.textContent.trim();
            }
        });
    });

    // --- Range Input Sync Logic ---
    function setupRangeSync(inputEl, valEl, fractionDigits) {
        if (!inputEl || !valEl) return;
        const updateVal = () => {
            valEl.textContent = Number(inputEl.value).toFixed(fractionDigits);
        };
        inputEl.addEventListener('input', updateVal);
        updateVal();
    }
    
    setupRangeSync(elPetScale, valPetScale, 1);
    setupRangeSync(elPetOpacity, valPetOpacity, 2);
    setupRangeSync(elPetHoverOpacity, valPetHoverOpacity, 2);
    // celebration secs uses integer display with 's' suffix
    if (elCelebrationSecs && valCelebrationSecs) {
        const updateCelebVal = () => { valCelebrationSecs.textContent = elCelebrationSecs.value + 's'; };
        elCelebrationSecs.addEventListener('input', updateCelebVal);
        updateCelebVal();
    }
    if (elSleepTimeoutSecs && valSleepTimeoutSecs) {
        const updateSleepVal = () => { valSleepTimeoutSecs.textContent = elSleepTimeoutSecs.value + 's'; };
        elSleepTimeoutSecs.addEventListener('input', updateSleepVal);
        updateSleepVal();
    }

    // --- Auto-Save Logic ---

    function showAutoSaveState(state) {
        // state: 'saving' | 'saved' | 'error' — surfaced through the
        // standard toast notification channel.
        const lang = elLanguage ? elLanguage.value : 'zh-CN';
        if (state === 'saving') {
            showToast(t('msg_auto_saving', lang));
        } else if (state === 'saved') {
            showToast(t('msg_auto_saved', lang), 'success');
        } else if (state === 'error') {
            showToast(t('msg_auto_save_error', lang), 'error');
        }
    }

    async function performSave() {
        const lang = elLanguage.value;

        if (!currentConfig.general) currentConfig.general = {};
        if (!currentConfig.api) currentConfig.api = {};
        if (!currentConfig.renderer) currentConfig.renderer = {};
        if (!currentConfig.renderer['desktop-pet']) currentConfig.renderer['desktop-pet'] = {};
        if (!currentConfig.hooks) currentConfig.hooks = {};
        if (!currentConfig.runtime) currentConfig.runtime = {};
        if (!currentConfig.remote) currentConfig.remote = {};
        if (!currentConfig.sessions) currentConfig.sessions = { hidden_sessions: [] };

        currentConfig.general.language = elLanguage.value;
        currentConfig.api.port = parseInt(elApiPort.value, 10);
        currentConfig.runtime.mode = elRuntimeMode.value === 'remote' ? 'remote' : 'local';

        const remoteEndpoint = elRemoteEndpoint.value.trim();
        const remotePath = elRemotePath.value.trim();
        const remoteToken = elRemoteToken.value.trim();
        const boundedInteger = (input, fallback, min, max) => {
            const value = Number.parseInt(input.value, 10);
            if (!Number.isFinite(value)) return fallback;
            return Math.min(max, Math.max(min, value));
        };
        const reconnectInitial = boundedInteger(elRemoteReconnectInitial, 1, 1, 3600);
        const reconnectMax = boundedInteger(elRemoteReconnectMax, 30, reconnectInitial, 86400);
        currentConfig.remote.endpoint = remoteEndpoint || null;
        currentConfig.remote.path = remotePath || '/api/v1/state-stream';
        currentConfig.remote.tls = elRemoteTls.checked;
        if (remoteToken) {
            const tokenPath = await invoke('save_remote_token', { token: remoteToken });
            currentConfig.remote.token_file = tokenPath;
            // Never leave the secret in the WebView after it has been handed
            // to the Rust side for persistence.
            elRemoteToken.value = '';
            updateRemoteTokenPlaceholder();
        }
        currentConfig.remote.connect_timeout_secs = boundedInteger(elRemoteConnectTimeout, 10, 1, 600);
        currentConfig.remote.reconnect_initial_secs = reconnectInitial;
        currentConfig.remote.reconnect_max_secs = reconnectMax;
        elRemoteConnectTimeout.value = currentConfig.remote.connect_timeout_secs;
        elRemoteReconnectInitial.value = reconnectInitial;
        elRemoteReconnectMax.value = reconnectMax;

        currentConfig.renderer['desktop-pet'].scale = parseFloat(elPetScale.value);
        currentConfig.renderer['desktop-pet'].always_on_top = elPetAlwaysTop.checked;
        currentConfig.renderer['desktop-pet'].show_on_all_desktops = elPetAllDesktops.checked;
        currentConfig.renderer['desktop-pet'].click_through = elPetClickThrough.checked;
        currentConfig.renderer['desktop-pet'].hide_on_hover = elPetHideOnHover.checked;
        currentConfig.renderer['desktop-pet'].show_window_frame = elPetWindowFrame.checked;
        currentConfig.renderer['desktop-pet'].opacity = parseFloat(elPetOpacity.value);
        currentConfig.renderer['desktop-pet'].hover_opacity = parseFloat(elPetHoverOpacity.value);
        currentConfig.renderer['desktop-pet'].snap_to_corner = elPetSnapCorner.checked;
        currentConfig.renderer['desktop-pet'].show_task_bubble = elShowBubble.checked;
        currentConfig.renderer['desktop-pet'].show_pet = elShowPet.checked;
        currentConfig.renderer['desktop-pet'].show_dashboard = elShowStats.checked;
        currentConfig.renderer['desktop-pet'].dashboard_style = elDashboardStyle.value;
        currentConfig.renderer['desktop-pet'].dashboard_position = elDashboardPosition.value;
        currentConfig.renderer['desktop-pet'].dashboard_layout = elDashboardLayout.value;
        currentConfig.renderer['desktop-pet'].dashboard_alignment = elDashboardAlignment.value;
        currentConfig.renderer['desktop-pet'].celebration_secs = parseInt(elCelebrationSecs.value, 10);
        if (elSleepTimeoutSecs) {
            currentConfig.renderer['desktop-pet'].sleep_timeout_secs = parseInt(elSleepTimeoutSecs.value, 10);
        }
        // Drop an empty mapping so no empty `[..event_status_map]` section is
        // written to the TOML file on save.
        if (currentConfig.renderer['desktop-pet'].event_status_map &&
            Object.keys(currentConfig.renderer['desktop-pet'].event_status_map).length === 0) {
            delete currentConfig.renderer['desktop-pet'].event_status_map;
        }

        currentConfig.hooks.socket_path = elUdsPath.value;
        currentConfig.hooks.tcp_port = parseInt(elTcpPort.value, 10);

        if (!currentConfig.cleanup) currentConfig.cleanup = {};
        currentConfig.cleanup.backup_files = elCleanupBackups.checked;
        currentConfig.cleanup.log_files = elCleanupLogs.checked;
        const cleanupAge = parseInt(elCleanupAgeDays.value, 10);
        currentConfig.cleanup.age_days = (Number.isFinite(cleanupAge) && cleanupAge >= 0) ? cleanupAge : 0;

        if (!currentConfig.update) currentConfig.update = {};
        currentConfig.update.check_on_startup = elUpdateStartup.checked;
        currentConfig.update.interval = elUpdateInterval.value;

        try {
            await invoke('save_config', { config: currentConfig });
            return true;
        } catch (e) {
            console.error('Save failed:', e);
            return false;
        }
    }

    function scheduleAutoSave() {
        if (autoSaveTimer) clearTimeout(autoSaveTimer);
        showAutoSaveState('saving');
        autoSaveTimer = setTimeout(async () => {
            const ok = await performSave();
            showAutoSaveState(ok ? 'saved' : 'error');
        }, 500);
    }

    // --- Event → Pet Status Mapping ---
    const eventStatusTable = document.getElementById('event-status-table');
    const btnResetEventStatus = document.getElementById('btn-reset-event-status');

    function eventStatusMapConfig() {
        if (!currentConfig.renderer) currentConfig.renderer = {};
        if (!currentConfig.renderer['desktop-pet']) currentConfig.renderer['desktop-pet'] = {};
        if (!currentConfig.renderer['desktop-pet'].event_status_map) {
            currentConfig.renderer['desktop-pet'].event_status_map = {};
        }
        return currentConfig.renderer['desktop-pet'].event_status_map;
    }

    function renderEventStatusTable() {
        if (!eventStatusTable) return;
        const lang = elLanguage.value;
        const map = currentConfig.renderer?.['desktop-pet']?.event_status_map || {};

        const header = document.createElement('div');
        header.className = 'event-status-row event-status-header';
        header.innerHTML =
            `<span>${t('event_status_event_col', lang)}</span>` +
            `<span>${t('event_status_state_col', lang)}</span>`;

        const rows = EVENT_KINDS.map((kind) => {
            const row = document.createElement('div');
            row.className = 'event-status-row';

            const label = document.createElement('span');
            label.className = 'event-status-label';
            label.textContent = t('evt_' + kind, lang);

            const selectWrap = document.createElement('div');
            selectWrap.className = 'custom-select';
            const select = document.createElement('select');
            // Label the "default" option with the status the built-in behavior
            // actually resolves to, so it is not a mystery.
            const defStatus = EVENT_DEFAULT_STATUS[kind];
            const openParen = lang === 'zh-CN' ? '（' : ' (';
            const closeParen = lang === 'zh-CN' ? '）' : ')';
            const defDetail = defStatus
                ? t('status_' + defStatus.replace(/-/g, '_'), lang)
                : t('status_default_noop', lang);
            const defOpt = document.createElement('option');
            defOpt.value = 'default';
            defOpt.textContent = t('status_default', lang) + openParen + defDetail + closeParen;
            select.appendChild(defOpt);
            for (const opt of EVENT_STATUS_OPTIONS) {
                const option = document.createElement('option');
                option.value = opt;
                // Option values are kebab-case (matching config TOML) but i18n
                // keys are snake_case, so normalize before looking up the label.
                option.textContent = t('status_' + opt.replace(/-/g, '_'), lang);
                select.appendChild(option);
            }
            select.value = EVENT_STATUS_OPTIONS.includes(map[kind]) ? map[kind] : 'default';
            select.addEventListener('change', () => setEventStatus(kind, select.value));

            selectWrap.appendChild(select);
            row.appendChild(label);
            row.appendChild(selectWrap);
            return row;
        });

        eventStatusTable.replaceChildren(header, ...rows);
    }

    function setEventStatus(kind, value) {
        const map = eventStatusMapConfig();
        if (value === 'default') {
            delete map[kind];
        } else {
            map[kind] = value;
        }
        if (Object.keys(map).length === 0) {
            delete currentConfig.renderer['desktop-pet'].event_status_map;
        }
        scheduleAutoSave();
    }

    function resetEventStatus() {
        delete currentConfig.renderer?.['desktop-pet']?.event_status_map;
        renderEventStatusTable();
        scheduleAutoSave();
    }

    // --- Config Load/Save Logic ---

    // Load config
    try {
        currentConfig = await invoke('get_config');
        
        // General
        if (currentConfig.general) {
            if (currentConfig.general.language) {
                elLanguage.value = currentConfig.general.language;
                applyTranslations(currentConfig.general.language);
            } else {
                applyTranslations('en-US'); // default
            }
        } else {
            applyTranslations('en-US');
        }
        if (currentConfig.api && currentConfig.api.port) {
            elApiPort.value = currentConfig.api.port;
        }

        const runtimeConfig = currentConfig.runtime || {};
        elRuntimeMode.value = runtimeConfig.mode === 'remote' ? 'remote' : 'local';
        const remoteConfig = currentConfig.remote || {};
        elRemoteEndpoint.value = remoteConfig.endpoint || '';
        elRemotePath.value = remoteConfig.path || '/api/v1/state-stream';
        elRemoteTls.checked = remoteConfig.tls === true;
        elRemoteToken.value = '';
        updateRemoteTokenPlaceholder();
        elRemoteConnectTimeout.value = remoteConfig.connect_timeout_secs ?? 10;
        elRemoteReconnectInitial.value = remoteConfig.reconnect_initial_secs ?? 1;
        elRemoteReconnectMax.value = remoteConfig.reconnect_max_secs ?? 30;
        updateRemoteSettingsVisibility();
        
        // Renderer Desktop Pet
        if (currentConfig.renderer && currentConfig.renderer['desktop-pet']) {
            const petConf = currentConfig.renderer['desktop-pet'];
            if (petConf.scale !== undefined) {
                elPetScale.value = petConf.scale;
                if (valPetScale) valPetScale.textContent = Number(petConf.scale).toFixed(1);
            }
            if (petConf.always_on_top !== undefined) elPetAlwaysTop.checked = petConf.always_on_top;
            if (petConf.show_on_all_desktops !== undefined) {
                elPetAllDesktops.checked = petConf.show_on_all_desktops;
            }
            if (petConf.click_through !== undefined) {
                elPetClickThrough.checked = petConf.click_through;
            }
            if (petConf.hide_on_hover !== undefined) {
                elPetHideOnHover.checked = petConf.hide_on_hover;
            }
            if (petConf.show_window_frame !== undefined) {
                elPetWindowFrame.checked = petConf.show_window_frame;
            }
            if (petConf.opacity !== undefined) {
                elPetOpacity.value = petConf.opacity;
                if (valPetOpacity) valPetOpacity.textContent = Number(petConf.opacity).toFixed(2);
            }
            if (petConf.hover_opacity !== undefined) {
                elPetHoverOpacity.value = petConf.hover_opacity;
                if (valPetHoverOpacity) valPetHoverOpacity.textContent = Number(petConf.hover_opacity).toFixed(2);
            }
            if (petConf.snap_to_corner !== undefined) {
                elPetSnapCorner.checked = petConf.snap_to_corner;
            }
            if (petConf.show_task_bubble !== undefined) elShowBubble.checked = petConf.show_task_bubble;
            if (petConf.show_pet !== undefined) elShowPet.checked = petConf.show_pet;
            if (petConf.show_dashboard !== undefined) elShowStats.checked = petConf.show_dashboard;
            elDashboardStyle.value = petConf.dashboard_style || 'classic';
            elDashboardPosition.value = petConf.dashboard_position || 'bottom';
            elDashboardLayout.value = petConf.dashboard_layout || 'vertical';
            elDashboardAlignment.value = petConf.dashboard_alignment || 'bottom';
            if (petConf.celebration_secs !== undefined && elCelebrationSecs) {
                elCelebrationSecs.value = petConf.celebration_secs;
                if (valCelebrationSecs) valCelebrationSecs.textContent = petConf.celebration_secs + 's';
            }
            if (petConf.sleep_timeout_secs !== undefined && elSleepTimeoutSecs) {
                elSleepTimeoutSecs.value = petConf.sleep_timeout_secs;
                if (valSleepTimeoutSecs) valSleepTimeoutSecs.textContent = petConf.sleep_timeout_secs + 's';
            }
        }

        renderEventStatusTable();

        // Hooks / IPC
        if (currentConfig.hooks) {
            if (currentConfig.hooks.socket_path) elUdsPath.value = currentConfig.hooks.socket_path;
            if (currentConfig.hooks.tcp_port) elTcpPort.value = currentConfig.hooks.tcp_port;
        }

        // Data cleanup
        if (currentConfig.cleanup) {
            if (currentConfig.cleanup.backup_files !== undefined) {
                elCleanupBackups.checked = currentConfig.cleanup.backup_files;
            }
            if (currentConfig.cleanup.log_files !== undefined) {
                elCleanupLogs.checked = currentConfig.cleanup.log_files;
            }
            if (currentConfig.cleanup.age_days !== undefined) {
                elCleanupAgeDays.value = currentConfig.cleanup.age_days;
            }
        }

        // Update check settings
        if (currentConfig.update) {
            if (currentConfig.update.check_on_startup !== undefined) {
                elUpdateStartup.checked = currentConfig.update.check_on_startup;
            }
            elUpdateInterval.value = currentConfig.update.interval || 'daily';
        } else {
            elUpdateStartup.checked = true;
            elUpdateInterval.value = 'daily';
        }

        // Dynamic version display (About + Update group) instead of a
        // hardcoded value that goes stale between releases.
        try {
            const appVersion = await invoke('get_app_version');
            const versionText = 'v' + appVersion;
            if (aboutVersion) aboutVersion.textContent = versionText;
            if (elUpdateCurrentVersion) elUpdateCurrentVersion.textContent = versionText;
        } catch (e) {
            console.error('Failed to load app version', e);
        }

        // Load initial active sessions
        try {
            const activeSessions = await invoke('get_active_sessions');
            renderSessionList(activeSessions);
        } catch (e) {
            console.error("Failed to load active sessions", e);
        }

        // Listen for state changes to update active sessions dynamically
        try {
            const appWin = getCurrentWebviewWindow();
            if (appWin && appWin.listen) {
                appWin.listen('settings_state_changed', (event) => {
                    if (event.payload && Array.isArray(event.payload.agents)) {
                        renderSessionList(event.payload.agents);
                    }
                });
            }

            // Show the update prompt when the backend reports an available
            // release (startup auto-check or tray "Check for Updates").
            if (appWin && appWin.listen) {
                appWin.listen('update_available', (event) => {
                    if (event.payload && event.payload.has_update) {
                        openUpdateModal(event.payload);
                    }
                });
            }

            // Keep the cached position in sync with live drags. Position has no
            // settings control; without this a settings save would revert a
            // freshly dragged position to the value cached when settings opened.
            if (appWin && appWin.listen) {
                appWin.listen('config_changed', (event) => {
                    const pet = event.payload && event.payload.renderer
                        && event.payload.renderer['desktop-pet'];
                    if (pet && currentConfig && currentConfig.renderer
                        && currentConfig.renderer['desktop-pet']) {
                        currentConfig.renderer['desktop-pet'].position = pet.position;
                    }
                });
            }
        } catch (e) {
            console.error("Failed to listen on webview window", e);
        }

        // Fallback for a settings window created after a startup check ran:
        // the event may have been missed, so claim the stashed result.
        try {
            const pendingUpdate = await invoke('get_pending_update');
            if (pendingUpdate && pendingUpdate.has_update) {
                openUpdateModal(pendingUpdate);
            }
        } catch (e) {
            console.error("Failed to fetch pending update", e);
        }

    } catch (e) {
        console.error("Failed to load config", e);
    }

    // Load sprite packs (must be after DOM and config are ready)
    await loadAndRenderSpritePacks();
    
    async function loadAndRenderSpritePacks() {
        if (!spritePackGrid) return;
        try {
            const packs = await invoke('get_sprite_packs');

            // Determine active sprite: prefer config, then query backend, then default
            let activeSprite = currentConfig.renderer?.['desktop-pet']?.sprite;
            if (!activeSprite) {
                try {
                    const activePack = await invoke('get_active_sprite_pack');
                    activeSprite = activePack?.manifest?.id || 'british-blue';
                } catch (_) {
                    activeSprite = 'british-blue';
                }
            }

            const lang = elLanguage ? elLanguage.value : 'zh-CN';

            spritePackGrid.innerHTML = '';

            if (!packs || packs.length === 0) {
                spritePackGrid.innerHTML = `<div style="color:var(--text-muted);font-size:13px;padding:16px;">未找到可用的素材包</div>`;
                return;
            }

            packs.forEach(pack => {
                const manifest = pack.manifest;
                const isActive = manifest.id === activeSprite;

                let previewSrc = '';
                const previewFile = manifest.preview || 'idle.png';
                if (pack.is_builtin) {
                    previewSrc = `/sprites/${manifest.id}/${previewFile}`;
                } else {
                    previewSrc = convertFileSrc(`${pack.path}/${previewFile}`);
                }

                const card = document.createElement('div');
                card.className = `sprite-pack-card ${isActive ? 'active' : ''}`;

                const badgeClass = pack.is_builtin ? 'builtin' : 'custom';
                const badgeText = pack.is_builtin ? t('lbl_builtin', lang) : t('lbl_custom', lang);
                const buttonText = isActive ? t('lbl_in_use', lang) : t('btn_use_pack', lang);

                card.innerHTML = `
                    <div class="sprite-pack-header">
                        <div class="sprite-pack-preview">
                            <img src="${previewSrc}" alt="${manifest.name}" onerror="this.style.opacity='0.2'" />
                        </div>
                        <div class="sprite-pack-details">
                            <div class="sprite-pack-title" title="${manifest.name}">${manifest.name}</div>
                            <div class="sprite-pack-author">${t('lbl_pack_author', lang)}: ${manifest.author || 'Unknown'}</div>
                        </div>
                    </div>
                    <div class="sprite-pack-desc">${manifest.description || ''}</div>
                    <div class="sprite-pack-meta-row">
                        <span class="sprite-pack-badge ${badgeClass}">${badgeText}</span>
                        ${manifest.created_at ? `<span>${t('lbl_pack_created', lang)}: ${manifest.created_at}</span>` : ''}
                        ${manifest.email ? `<span>${t('lbl_pack_email', lang)}: ${manifest.email}</span>` : ''}
                    </div>
                    <div class="sprite-pack-footer" style="display:flex; gap:8px;">
                        <button class="secondary-btn btn-sm btn-preview-pack" style="white-space:nowrap; display:inline-flex; align-items:center; gap:4px; padding: 6px 10px;">
                            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
                                <circle cx="12" cy="12" r="3"></circle>
                            </svg>
                            <span data-i18n="btn_preview_pack">${t('btn_preview_pack', lang)}</span>
                        </button>
                        <button class="sprite-pack-use-btn ${isActive ? 'in-use' : 'secondary-btn'}" ${isActive ? 'disabled' : ''} style="flex:1;">
                            ${buttonText}
                        </button>
                    </div>
                `;

                const imgEl = card.querySelector('.sprite-pack-preview img');
                if (imgEl) {
                    imgEl.onerror = () => {
                        if (pack.path) {
                            imgEl.src = convertFileSrc(`${pack.path}/${previewFile}`);
                        } else {
                            imgEl.style.opacity = '0.2';
                        }
                    };
                }

                const previewBtn = card.querySelector('.btn-preview-pack');
                if (previewBtn) {
                    previewBtn.addEventListener('click', (e) => {
                        e.stopPropagation();
                        openPreviewModal(pack);
                    });
                }

                if (!isActive) {
                    const useBtn = card.querySelector('.sprite-pack-use-btn');
                    useBtn.addEventListener('click', () => {
                        if (!currentConfig.renderer) currentConfig.renderer = {};
                        if (!currentConfig.renderer['desktop-pet']) currentConfig.renderer['desktop-pet'] = {};
                        currentConfig.renderer['desktop-pet'].sprite = manifest.id;
                        loadAndRenderSpritePacks();
                        scheduleAutoSave();
                    });
                }

                spritePackGrid.appendChild(card);
            });
        } catch (e) {
            console.error("Failed to load sprite packs:", e);
        }
    }

    function openPreviewModal(pack) {
        if (!previewModalOverlay || !pack || !pack.manifest) return;
        const manifest = pack.manifest;
        const lang = elLanguage ? elLanguage.value : 'zh-CN';

        if (previewModalSubInfo) {
            previewModalSubInfo.innerHTML = `
                <span class="pack-name">${manifest.name}</span>
                <span>v${manifest.version || '1.0.0'}</span>
                <span>(${pack.is_builtin ? t('lbl_builtin', lang) : t('lbl_custom', lang)})</span>
            `;
        }

        if (previewModalStateGrid) {
            previewModalStateGrid.innerHTML = '';
            const states = manifest.states || {};

            const stateLabels = {
                'idle': t('state_idle', lang),
                'working': t('state_working', lang),
                'thinking': t('state_thinking', lang),
                'interacting': t('state_interacting', lang),
                'happy': t('state_interacting', lang),
                'celebrating': t('state_celebrating', lang),
                'alarmed': t('state_alarmed', lang),
                'sleeping': t('state_sleeping', lang),
                'watching': t('state_watching', lang),
            };

            const standardOrder = ['idle', 'working', 'thinking', 'interacting', 'happy', 'celebrating', 'alarmed', 'sleeping', 'watching'];

            const sortedEntries = Object.entries(states).sort(([a], [b]) => {
                let idxA = standardOrder.indexOf(a);
                let idxB = standardOrder.indexOf(b);
                if (idxA === -1) idxA = 999;
                if (idxB === -1) idxB = 999;
                if (idxA !== idxB) return idxA - idxB;
                return a.localeCompare(b);
            });

            sortedEntries.forEach(([stateKey, fileName]) => {
                let imgSrc = '';
                if (pack.is_builtin) {
                    imgSrc = `/sprites/${manifest.id}/${fileName}`;
                } else {
                    imgSrc = convertFileSrc(`${pack.path}/${fileName}`);
                }

                const displayName = stateLabels[stateKey] || stateKey;

                const item = document.createElement('div');
                item.className = 'preview-state-item';
                item.innerHTML = `
                    <div class="preview-state-box">
                        <img src="${imgSrc}" alt="${stateKey}" />
                    </div>
                    <div class="preview-state-name">${displayName}</div>
                    <div class="preview-state-filename">${fileName}</div>
                `;

                const stateImg = item.querySelector('img');
                if (stateImg) {
                    stateImg.onerror = () => {
                        if (pack.path) {
                            stateImg.src = convertFileSrc(`${pack.path}/${fileName}`);
                        } else {
                            stateImg.style.opacity = '0.2';
                        }
                    };
                }

                previewModalStateGrid.appendChild(item);
            });
        }

        previewModalOverlay.classList.add('active');
    }

    function closePreviewModal() {
        if (previewModalOverlay) {
            previewModalOverlay.classList.remove('active');
        }
    }

    if (btnClosePreviewModal) {
        btnClosePreviewModal.addEventListener('click', closePreviewModal);
    }
    if (previewModalOverlay) {
        previewModalOverlay.addEventListener('click', (e) => {
            if (e.target === previewModalOverlay) {
                closePreviewModal();
            }
        });
    }

    if (btnOpenSpriteDir) {
        btnOpenSpriteDir.addEventListener('click', async () => {
            try {
                await invoke('open_sprite_dir');
            } catch (e) {
                console.error("Failed to open sprite directory:", e);
            }
        });
    }

    if (btnImportPack) {
        btnImportPack.addEventListener('click', async () => {
            const lang = elLanguage ? elLanguage.value : 'zh-CN';
            try {
                const imported = await invoke('import_sprite_pack', { path: null });
                if (imported && imported.manifest) {
                    if (!currentConfig.renderer) currentConfig.renderer = {};
                    if (!currentConfig.renderer['desktop-pet']) currentConfig.renderer['desktop-pet'] = {};
                    currentConfig.renderer['desktop-pet'].sprite = imported.manifest.id;
                    await loadAndRenderSpritePacks();
                    scheduleAutoSave();
                }
            } catch (err) {
                if (err !== 'Cancelled' && err !== 'User cancelled selection') {
                    alert(t('msg_import_failed', lang) + err);
                }
            }
        });
    }

    // Language setting update UI + auto save
    elLanguage.addEventListener('change', () => {
        applyTranslations(elLanguage.value);
        updateRemoteTokenPlaceholder();
        loadAndRenderSpritePacks();
        renderEventStatusTable();
        scheduleAutoSave();
    });

    const btnOpenSysLoginItems = document.getElementById('btn-open-sys-login-items');
    if (btnOpenSysLoginItems) {
        btnOpenSysLoginItems.addEventListener('click', async () => {
            try {
                await invoke('open_login_items_settings');
            } catch (e) {
                console.error("Failed to open system settings:", e);
            }
        });
    }

    // Bind all controls for auto-save
    const autoSaveControls = [
        elRuntimeMode, elRemoteTls,
        elPetAlwaysTop, elPetAllDesktops, elPetClickThrough, elPetHideOnHover, elPetWindowFrame, elPetSnapCorner, elShowBubble, elShowPet, elShowStats,
        elDashboardStyle, elDashboardPosition, elDashboardLayout, elDashboardAlignment,
        elCleanupBackups, elCleanupLogs,
        elUpdateStartup, elUpdateInterval
    ];
    autoSaveControls.forEach(el => {
        if (el) el.addEventListener('change', scheduleAutoSave);
    });

    const autoSaveInputs = [
        elRemoteEndpoint, elRemotePath, elRemoteToken, elRemoteConnectTimeout,
        elRemoteReconnectInitial, elRemoteReconnectMax,
        elApiPort, elPetScale, elPetOpacity, elPetHoverOpacity, elUdsPath, elTcpPort, elCelebrationSecs, elSleepTimeoutSecs,
        elCleanupAgeDays
    ];
    autoSaveInputs.forEach(el => {
        if (el) el.addEventListener('change', scheduleAutoSave);
    });

    if (elRuntimeMode) {
        elRuntimeMode.addEventListener('change', () => {
            updateRemoteSettingsVisibility();
            scheduleAutoSave();
        });
    }

    if (btnResetEventStatus) {
        btnResetEventStatus.addEventListener('click', resetEventStatus);
    }

    // Manual save button (fires immediately, cancels any pending auto-save)
    saveBtn.addEventListener('click', async () => {
        if (autoSaveTimer) { clearTimeout(autoSaveTimer); autoSaveTimer = null; }
        saveBtn.disabled = true;
        showAutoSaveState('saving');
        const ok = await performSave();
        showAutoSaveState(ok ? 'saved' : 'error');
        saveBtn.disabled = false;
    });

    // --- Hooks management (opened in a dedicated window) ---
    const cleanupModal = document.getElementById('cleanup-modal');
    const btnCleanupCancel = document.getElementById('btn-cleanup-cancel');
    const btnCleanupConfirm = document.getElementById('btn-cleanup-confirm');

    const updateModal = document.getElementById('update-modal');
    const btnUpdateLater = document.getElementById('btn-update-later');
    const btnUpdateIgnore = document.getElementById('btn-update-ignore');
    const btnUpdateSkip = document.getElementById('btn-update-skip');
    const btnUpdateDownload = document.getElementById('btn-update-download');

    // The full hook management panel (inject/uninstall/config-path/details/
    // test) is mounted here from the shared hook-panel module.
    mountHookPanel(document.getElementById('hook-status-list'), elLanguage.value);

    // Standard notification channel for the settings panel: a single global
    // toast element is created lazily and re-shown for each transient status
    // (auto-save progress, hook test results, etc.). Types: 'success' (green),
    // 'error' (red); omit for neutral progress messages.
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


    // --- Data Cleanup Logic ---

    function formatBytes(bytes) {
        if (bytes < 1024) return bytes + ' B';
        if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
        if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
        return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB';
    }

    let currentCleanupPreview = null;

    // Preview first (dry run) so the modal can confirm what will be deleted.
    btnCleanupRun.addEventListener('click', async () => {
        const lang = elLanguage.value;
        try {
            const summary = await invoke('run_data_cleanup', { dryRun: true });
            if (summary.backup_count === 0 && summary.log_count === 0) {
                showToast(t('msg_cleanup_nothing', lang), 'success');
                return;
            }
            currentCleanupPreview = summary;
            document.getElementById('cleanup-summary-backup-count').textContent = summary.backup_count;
            document.getElementById('cleanup-summary-log-count').textContent = summary.log_count;
            document.getElementById('cleanup-summary-freed-bytes').textContent = formatBytes(summary.freed_bytes);
            cleanupModal.style.display = 'flex';
        } catch (e) {
            console.error('Cleanup preview failed', e);
            showToast(t('msg_cleanup_preview_failed', lang), 'error');
        }
    });

    btnCleanupCancel.addEventListener('click', () => {
        cleanupModal.style.display = 'none';
        currentCleanupPreview = null;
    });

    btnCleanupConfirm.addEventListener('click', async () => {
        const lang = elLanguage.value;
        if (!currentCleanupPreview) return;
        try {
            btnCleanupConfirm.disabled = true;
            const result = await invoke('run_data_cleanup', { dryRun: false });
            cleanupModal.style.display = 'none';
            const total = result.backup_count + result.log_count;
            showToast(
                t('msg_cleanup_done', lang) + ' ' + total + ' ' + t('msg_cleanup_files', lang) +
                ' · ' + formatBytes(result.freed_bytes),
                'success'
            );
            if (result.failures && result.failures.length > 0) {
                showToast(t('msg_cleanup_partial', lang) + ' (' + result.failures.length + ')', 'error');
            }
        } catch (e) {
            console.error('Cleanup failed', e);
            showToast(t('msg_cleanup_failed', lang), 'error');
        } finally {
            btnCleanupConfirm.disabled = false;
            currentCleanupPreview = null;
        }
    });

    // --- Update Check Logic ---

    let currentUpdateResult = null;

    // Idempotent: a startup check, tray action, manual check and the pending
    // fallback can all trigger the same version; do not stack or re-open it.
    function openUpdateModal(result) {
        if (updateModal.style.display === 'flex' &&
            currentUpdateResult &&
            currentUpdateResult.latest_version === result.latest_version) {
            return;
        }
        currentUpdateResult = result;
        document.getElementById('update-version-current').textContent = 'v' + result.current_version;
        document.getElementById('update-version-latest').textContent = 'v' + (result.latest_version || '');
        document.getElementById('update-release-date').textContent = result.published_at || '-';
        document.getElementById('update-release-notes').textContent = result.release_notes || '';
        updateModal.style.display = 'flex';
    }

    function closeUpdateModal() {
        updateModal.style.display = 'none';
        currentUpdateResult = null;
    }

    // Run the update check and surface the result (update modal, or a toast
    // for up-to-date / skipped / ignored / failure). Shared by the manual
    // button and the tray "Check for Updates" (which opens this window with
    // the check requested).
    async function runUpdateCheck() {
        const lang = elLanguage.value;
        btnCheckUpdate.disabled = true;
        try {
            const result = await invoke('check_for_updates', { force: true });
            if (result.has_update) {
                openUpdateModal(result);
            } else if (result.suppressed_reason === 'skipped') {
                showToast(t('msg_update_skipped', lang), 'success');
            } else if (result.suppressed_reason === 'ignored') {
                showToast(t('msg_update_ignored', lang), 'success');
            } else {
                showToast(t('msg_update_latest', lang), 'success');
            }
        } catch (e) {
            console.error('Update check failed', e);
            showToast(t('msg_update_check_failed', lang), 'error');
        } finally {
            btnCheckUpdate.disabled = false;
        }
    }
    btnCheckUpdate.addEventListener('click', runUpdateCheck);

    btnUpdateLater.addEventListener('click', closeUpdateModal);

    btnUpdateDownload.addEventListener('click', async () => {
        if (!currentUpdateResult) return;
        const url = currentUpdateResult.download_url || currentUpdateResult.release_url;
        try {
            await invoke('open_url', { url });
        } catch (e) {
            console.error('Failed to open download URL', e);
        }
        closeUpdateModal();
    });

    btnUpdateSkip.addEventListener('click', async () => {
        const lang = elLanguage.value;
        if (!currentUpdateResult || !currentUpdateResult.latest_version) return;
        try {
            await invoke('skip_update', { version: currentUpdateResult.latest_version });
            showToast(t('msg_update_skipped', lang), 'success');
        } catch (e) {
            console.error('Skip update failed', e);
            showToast(t('msg_update_check_failed', lang), 'error');
        }
        closeUpdateModal();
    });

    btnUpdateIgnore.addEventListener('click', async () => {
        const lang = elLanguage.value;
        if (!currentUpdateResult || !currentUpdateResult.latest_version) return;
        try {
            await invoke('ignore_update', { version: currentUpdateResult.latest_version });
            showToast(t('msg_update_ignored', lang), 'success');
        } catch (e) {
            console.error('Ignore update failed', e);
            showToast(t('msg_update_check_failed', lang), 'error');
        }
        closeUpdateModal();
    });

});
