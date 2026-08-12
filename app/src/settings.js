import { applyTranslations, t } from './i18n.js';

const { invoke } = window.__TAURI__.core;
const { getCurrentWebviewWindow } = window.__TAURI__.webviewWindow;

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
    
    const elPetScale = document.getElementById('setting-pet-scale');
    const valPetScale = document.getElementById('val-pet-scale');
    const elPetAlwaysTop = document.getElementById('setting-pet-always-top');
    const elPetAllDesktops = document.getElementById('setting-pet-all-desktops');
    const elPetOpacity = document.getElementById('setting-pet-opacity');
    const valPetOpacity = document.getElementById('val-pet-opacity');
    const elShowBubble = document.getElementById('setting-show-bubble');
    const elShowPet = document.getElementById('setting-show-pet');
    const elShowStats = document.getElementById('setting-show-stats');
    const elDashboardStyle = document.getElementById('setting-dashboard-style');
    const elDashboardPosition = document.getElementById('setting-dashboard-position');
    const elDashboardLayout = document.getElementById('setting-dashboard-layout');
    const elDashboardAlignment = document.getElementById('setting-dashboard-alignment');

    const elUdsPath = document.getElementById('setting-uds-path');
    const elTcpPort = document.getElementById('setting-tcp-port');

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
        if (!currentConfig.sessions) currentConfig.sessions = { hidden_sessions: [] };

        currentConfig.general.language = elLanguage.value;
        currentConfig.api.port = parseInt(elApiPort.value, 10);

        currentConfig.renderer['desktop-pet'].scale = parseFloat(elPetScale.value);
        currentConfig.renderer['desktop-pet'].always_on_top = elPetAlwaysTop.checked;
        currentConfig.renderer['desktop-pet'].show_on_all_desktops = elPetAllDesktops.checked;
        currentConfig.renderer['desktop-pet'].opacity = parseFloat(elPetOpacity.value);
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

        currentConfig.hooks.socket_path = elUdsPath.value;
        currentConfig.hooks.tcp_port = parseInt(elTcpPort.value, 10);

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
            if (petConf.opacity !== undefined) {
                elPetOpacity.value = petConf.opacity;
                if (valPetOpacity) valPetOpacity.textContent = Number(petConf.opacity).toFixed(2);
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

        // Hooks / IPC
        if (currentConfig.hooks) {
            if (currentConfig.hooks.socket_path) elUdsPath.value = currentConfig.hooks.socket_path;
            if (currentConfig.hooks.tcp_port) elTcpPort.value = currentConfig.hooks.tcp_port;
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
        } catch (e) {
            console.error("Failed to listen on webview window", e);
        }

    } catch (e) {
        console.error("Failed to load config", e);
    }

    // Load sprite packs (must be after DOM and config are ready)
    await loadAndRenderSpritePacks();
    
    // Copy buttons logic
    document.querySelectorAll('.modal-path-copy').forEach(btn => {
        btn.addEventListener('click', async () => {
            const targetId = btn.getAttribute('data-target');
            const targetEl = document.getElementById(targetId);
            if (targetEl && targetEl.textContent) {
                try {
                    await navigator.clipboard.writeText(targetEl.textContent);
                    const origText = btn.textContent;
                    btn.textContent = t('btn_copied', elLanguage.value);
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
        loadAndRenderSpritePacks();
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
        elPetAlwaysTop, elPetAllDesktops, elShowBubble, elShowPet, elShowStats,
        elDashboardStyle, elDashboardPosition, elDashboardLayout, elDashboardAlignment
    ];
    autoSaveControls.forEach(el => {
        if (el) el.addEventListener('change', scheduleAutoSave);
    });

    const autoSaveInputs = [
        elApiPort, elPetScale, elPetOpacity, elUdsPath, elTcpPort, elCelebrationSecs, elSleepTimeoutSecs
    ];
    autoSaveInputs.forEach(el => {
        if (el) el.addEventListener('change', scheduleAutoSave);
    });

    // Manual save button (fires immediately, cancels any pending auto-save)
    saveBtn.addEventListener('click', async () => {
        if (autoSaveTimer) { clearTimeout(autoSaveTimer); autoSaveTimer = null; }
        saveBtn.disabled = true;
        showAutoSaveState('saving');
        const ok = await performSave();
        showAutoSaveState(ok ? 'saved' : 'error');
        saveBtn.disabled = false;
    });

    // --- Hooks Injection Logic ---
    const hookModal = document.getElementById('hook-modal');
    const btnModalCancel = document.getElementById('btn-modal-cancel');
    const btnModalConfirm = document.getElementById('btn-modal-confirm');
    const injectBeforeCode = document.getElementById('inject-before-code');
    const injectAfterCode = document.getElementById('inject-after-code');
    const uninstallModal = document.getElementById('uninstall-modal');
    const btnUninstallCancel = document.getElementById('btn-uninstall-cancel');
    const btnUninstallConfirm = document.getElementById('btn-uninstall-confirm');
    const uninstallBeforeCode = document.getElementById('uninstall-before-code');
    const uninstallAfterCode = document.getElementById('uninstall-after-code');

    const configViewerModal = document.getElementById('config-viewer-modal');
    const btnConfigViewerClose = document.getElementById('btn-config-viewer-close');
    const configViewerCode = document.getElementById('config-viewer-code');
    
    let currentInjectingAgent = null;
    let currentUninstallingAgent = null;
    let hooksStatusCache = {};
    let hookDetailsCache = {};     // agent -> AgentHookDetail (lazy)
    let expandedAgents = new Set(); // track which agent cards are expanded

    function syntaxHighlightJSON(json) {
        if (typeof json != 'string') {
             json = JSON.stringify(json, undefined, 2);
        }
        json = json.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
        return json.replace(/("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g, function (match) {
            let cls = 'json-number';
            if (/^"/.test(match)) {
                if (/:$/.test(match)) {
                    cls = 'json-key';
                } else {
                    cls = 'json-string';
                }
            } else if (/true|false/.test(match)) {
                cls = 'json-boolean';
            } else if (/null/.test(match)) {
                cls = 'json-null';
            }
            return '<span class="' + cls + '">' + match + '</span>';
        });
    }

    function renderDiff(beforeText, afterText, elBefore, elAfter) {
        const beforeLines = beforeText ? beforeText.split('\n') : [];
        const afterLines = afterText ? afterText.split('\n') : [];
        
        let beforeHTML = '';
        let afterHTML = '';
        
        let maxLines = Math.max(beforeLines.length, afterLines.length);
        for(let i=0; i<maxLines; i++) {
            const bLine = i < beforeLines.length ? beforeLines[i] : null;
            const aLine = i < afterLines.length ? afterLines[i] : null;
            
            if (bLine === aLine) {
                beforeHTML += `<div class="diff-line">${bLine !== null ? syntaxHighlightJSON(bLine) : ''}</div>`;
                afterHTML += `<div class="diff-line">${aLine !== null ? syntaxHighlightJSON(aLine) : ''}</div>`;
            } else if (bLine !== null && aLine === null) {
                beforeHTML += `<div class="diff-line diff-remove">${syntaxHighlightJSON(bLine)}</div>`;
                afterHTML += `<div class="diff-line"></div>`;
            } else if (bLine === null && aLine !== null) {
                beforeHTML += `<div class="diff-line"></div>`;
                afterHTML += `<div class="diff-line diff-add">${syntaxHighlightJSON(aLine)}</div>`;
            } else {
                beforeHTML += `<div class="diff-line diff-remove">${syntaxHighlightJSON(bLine)}</div>`;
                afterHTML += `<div class="diff-line diff-add">${syntaxHighlightJSON(aLine)}</div>`;
            }
        }
        elBefore.innerHTML = beforeHTML;
        elAfter.innerHTML = afterHTML;
    }

    const AGENT_DISPLAY = {
        'antigravity': 'Antigravity',
        'claude-code': 'Claude Code',
        'codex': 'Codex',
        'qoder': 'Qoder',
    };

    const AGENTS = ['antigravity', 'claude-code', 'codex', 'qoder'];

    function renderAgentCards() {
        const container = document.getElementById('hook-status-list');
        if (!container) return;

        const lang = elLanguage ? elLanguage.value : 'zh-CN';

        container.innerHTML = '';

        AGENTS.forEach(agent => {
            const status = hooksStatusCache[agent];
            const isInjected = status ? status.injected : false;
            const isExpanded = expandedAgents.has(agent);

            // Build card
            const card = document.createElement('div');
            card.className = 'hook-agent-card';
            card.setAttribute('data-agent', agent);

            // --- Header row ---
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

            // Click on header (except buttons) toggles expand
            header.addEventListener('click', (e) => {
                if (e.target.closest('button')) return;
                toggleAgentExpand(agent);
            });

            card.appendChild(header);

            // --- Detail panel (rendered on expand) ---
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

            // --- Bind button handlers ---
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
                    renderDiff(diff.before, diff.after, injectBeforeCode, injectAfterCode);
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
                    renderDiff(diff.before, diff.after, uninstallBeforeCode, uninstallAfterCode);
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

    function renderHookPointsTable(detailEl, hookDetail, agent) {
        const lang = elLanguage ? elLanguage.value : 'zh-CN';
        const points = hookDetail.hook_points || [];

        if (points.length === 0) {
            detailEl.innerHTML = `<div class="hook-detail-empty">${t('lbl_no_hook_points', lang)}</div>`;
            return;
        }

        let html = '<div class="hook-points-table">';
        html += '<div class="hook-points-header">';
        html += `<span class="hook-col-event">${t('lbl_hook_event', lang)}</span>`;
        html += `<span class="hook-col-command">${t('lbl_hook_command', lang)}</span>`;
        html += `<span class="hook-col-test"></span>`;
        html += '</div>';

        points.forEach(pt => {
            const matcherLabel = pt.matcher ? ` (matcher: ${pt.matcher})` : '';
            // Prefer the full copy-pasteable test command (with mocked stdin
            // payload); fall back to the raw hook command for older payloads.
            const displayCmd = pt.test_command || pt.command;
            html += '<div class="hook-point-row">';
            html += `<span class="hook-col-event"><code>${pt.event_name}</code>${matcherLabel}</span>`;
            html += `<span class="hook-col-command"><code class="hook-cmd-text" title="${displayCmd.replace(/"/g, '&quot;')}">${displayCmd}</code></span>`;
            html += '<span class="hook-col-test">';
            html += `<button class="btn-test btn-test-bus" data-agent="${agent}" data-event="${pt.event_name}">${t('btn_test_eventbus', lang)}</button>`;
            html += `<button class="btn-test btn-copy-cmd">${t('btn_copy_command', lang)}</button>`;
            html += '</span>';
            html += '</div>';
        });

        html += '</div>';
        detailEl.innerHTML = html;

        // Bind event-bus test button handlers
        detailEl.querySelectorAll('.btn-test-bus').forEach(btn => {
            btn.addEventListener('click', async (e) => {
                e.stopPropagation();
                const evt = btn.getAttribute('data-event');
                await testHookPoint(agent, evt);
            });
        });

        // Bind copy-command button handlers (copies the full command text
        // from the same row so users can run it manually in their terminal)
        detailEl.querySelectorAll('.btn-copy-cmd').forEach(btn => {
            btn.addEventListener('click', async (e) => {
                e.stopPropagation();
                const row = btn.closest('.hook-point-row');
                const cmdEl = row ? row.querySelector('.hook-cmd-text') : null;
                const cmd = cmdEl ? cmdEl.textContent : '';
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

    async function toggleAgentExpand(agent) {
        const detailEl = document.getElementById(`hook-detail-${agent}`);
        if (!detailEl) return;

        const wasExpanded = expandedAgents.has(agent);

        if (wasExpanded) {
            expandedAgents.delete(agent);
            detailEl.classList.remove('expanded');
            // update arrow
            const card = detailEl.closest('.hook-agent-card');
            if (card) {
                const icon = card.querySelector('.hook-expand-icon');
                if (icon) icon.classList.remove('expanded');
            }
        } else {
            expandedAgents.add(agent);
            detailEl.classList.add('expanded');
            // update arrow
            const card = detailEl.closest('.hook-agent-card');
            if (card) {
                const icon = card.querySelector('.hook-expand-icon');
                if (icon) icon.classList.add('expanded');
            }

            // Lazy-load details if not cached
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

    async function testHookPoint(agent, eventName) {
        const lang = elLanguage ? elLanguage.value : 'zh-CN';

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

    // Initial load
    fetchHooksStatus();
});
