import { applyTranslations, t } from './i18n.js';

const { invoke } = window.__TAURI__.core;
const { getCurrentWebviewWindow } = window.__TAURI__.webviewWindow;

document.addEventListener('DOMContentLoaded', async () => {
    // Nav elements
    const menuItems = document.querySelectorAll('.menu-item');
    const contentTitle = document.getElementById('content-title');
    const contentScroll = document.getElementById('content-scroll');

    // Form elements
    const elLanguage = document.getElementById('setting-language');
    const elAutostart = document.getElementById('setting-autostart');
    const elApiPort = document.getElementById('setting-api-port');
    
    const elPetScale = document.getElementById('setting-pet-scale');
    const valPetScale = document.getElementById('val-pet-scale');
    const elPetAlwaysTop = document.getElementById('setting-pet-always-top');
    const elPetOpacity = document.getElementById('setting-pet-opacity');
    const valPetOpacity = document.getElementById('val-pet-opacity');
    const elShowBubble = document.getElementById('setting-show-bubble');
    const elShowPet = document.getElementById('setting-show-pet');
    const elShowStats = document.getElementById('setting-show-stats');

    const elUdsPath = document.getElementById('setting-uds-path');
    const elTcpPort = document.getElementById('setting-tcp-port');

    const saveBtn = document.getElementById('save-btn');
    const statusMsg = document.getElementById('save-status');

    let currentConfig = {};

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
            elAutostart.checked = currentConfig.general.auto_start !== false;
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
            if (petConf.opacity !== undefined) {
                elPetOpacity.value = petConf.opacity;
                if (valPetOpacity) valPetOpacity.textContent = Number(petConf.opacity).toFixed(2);
            }
            if (petConf.show_task_bubble !== undefined) elShowBubble.checked = petConf.show_task_bubble;
            if (petConf.show_pet !== undefined) elShowPet.checked = petConf.show_pet;
            if (petConf.show_dashboard !== undefined) elShowStats.checked = petConf.show_dashboard;
        }

        // Hooks / IPC
        if (currentConfig.hooks) {
            if (currentConfig.hooks.socket_path) elUdsPath.value = currentConfig.hooks.socket_path;
            if (currentConfig.hooks.tcp_port) elTcpPort.value = currentConfig.hooks.tcp_port;
        }

    } catch (e) {
        console.error("Failed to load config", e);
    }
    
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

    // Language setting update UI
    elLanguage.addEventListener('change', () => {
        applyTranslations(elLanguage.value);
    });

    // Save config
    saveBtn.addEventListener('click', async () => {
        const lang = elLanguage.value;
        saveBtn.textContent = t('msg_saving', lang);
        saveBtn.disabled = true;

        if (!currentConfig.general) currentConfig.general = {};
        if (!currentConfig.api) currentConfig.api = {};
        if (!currentConfig.renderer) currentConfig.renderer = {};
        if (!currentConfig.renderer['desktop-pet']) currentConfig.renderer['desktop-pet'] = {};
        if (!currentConfig.hooks) currentConfig.hooks = {};

        currentConfig.general.language = elLanguage.value;
        currentConfig.general.auto_start = elAutostart.checked;
        currentConfig.api.port = parseInt(elApiPort.value, 10);

        currentConfig.renderer['desktop-pet'].scale = parseFloat(elPetScale.value);
        currentConfig.renderer['desktop-pet'].always_on_top = elPetAlwaysTop.checked;
        currentConfig.renderer['desktop-pet'].opacity = parseFloat(elPetOpacity.value);
        currentConfig.renderer['desktop-pet'].show_task_bubble = elShowBubble.checked;
        currentConfig.renderer['desktop-pet'].show_pet = elShowPet.checked;
        currentConfig.renderer['desktop-pet'].show_dashboard = elShowStats.checked;

        currentConfig.hooks.socket_path = elUdsPath.value;
        currentConfig.hooks.tcp_port = parseInt(elTcpPort.value, 10);

        try {
            await invoke('save_config', { config: currentConfig });
            statusMsg.textContent = t('msg_saved', lang);
            statusMsg.style.opacity = '1';
            setTimeout(() => {
                statusMsg.style.opacity = '0';
            }, 3000);
        } catch (e) {
            statusMsg.textContent = t('msg_failed', lang) + e;
            statusMsg.style.color = '#FF3B30';
            statusMsg.style.opacity = '1';
        } finally {
            saveBtn.textContent = t('btn_save', lang);
            saveBtn.disabled = false;
        }
    });

    // --- Hooks Injection Logic ---
    const hookModal = document.getElementById('hook-modal');
    const btnModalCancel = document.getElementById('btn-modal-cancel');
    const btnModalConfirm = document.getElementById('btn-modal-confirm');
    const hookPreviewCode = document.getElementById('hook-preview-code');
    const hookModalPath = document.getElementById('hook-modal-path');
    const injectBeforeCode = document.getElementById('inject-before-code');
    const injectAfterCode = document.getElementById('inject-after-code');
    const uninstallModal = document.getElementById('uninstall-modal');
    const btnUninstallCancel = document.getElementById('btn-uninstall-cancel');
    const btnUninstallConfirm = document.getElementById('btn-uninstall-confirm');
    const uninstallBeforeCode = document.getElementById('uninstall-before-code');
    const uninstallAfterCode = document.getElementById('uninstall-after-code');

    const configViewerModal = document.getElementById('config-viewer-modal');
    const btnConfigViewerClose = document.getElementById('btn-config-viewer-close');
    const configViewerPath = document.getElementById('config-viewer-path');
    const configViewerCode = document.getElementById('config-viewer-code');
    
    let currentInjectingAgent = null;
    let currentUninstallingAgent = null;
    let hooksStatusCache = {};

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
        
        // Simple line-by-line diff with forced alignment
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

    async function fetchHooksStatus() {
        try {
            const status = await invoke('get_hooks_status');
            if (status) {
                hooksStatusCache = status;
                const agents = ['antigravity', 'claude-code', 'codex'];
                agents.forEach(agent => {
                    if (status[agent]) {
                        const isInj = status[agent].injected;
                        const badge = document.getElementById(`badge-${agent}`);
                        const btnInject = document.getElementById(`btn-inject-${agent}`);
                        const btnViewConfig = document.getElementById(`btn-view-config-${agent}`);
                        const btnUninstall = document.getElementById(`btn-uninstall-${agent}`);
                        
                        if (badge) {
                            badge.className = isInj ? 'badge badge-injected' : 'badge badge-not-injected';
                            badge.textContent = isInj ? t('badge_injected', elLanguage.value) : t('badge_not_injected', elLanguage.value);
                        }
                        if (btnInject) btnInject.style.display = isInj ? 'none' : 'inline-block';
                        if (btnViewConfig) btnViewConfig.style.display = isInj ? 'inline-block' : 'none';
                        if (btnUninstall) btnUninstall.style.display = isInj ? 'inline-block' : 'none';
                    }
                });
            }
        } catch (e) {
            console.error("Failed to fetch hooks status", e);
        }
    }

    const AGENTS = ['antigravity', 'claude-code', 'codex'];
    AGENTS.forEach(agent => {
        const btnViewConfig = document.getElementById(`btn-view-config-${agent}`);
        const btnInject = document.getElementById(`btn-inject-${agent}`);
        const btnUninstall = document.getElementById(`btn-uninstall-${agent}`);
        
        if (btnViewConfig) {
            btnViewConfig.addEventListener('click', async () => {
                try {
                    const content = await invoke('get_config_content', { agent });
                    if (hooksStatusCache && hooksStatusCache[agent] && hooksStatusCache[agent].config_path) {
                        document.getElementById('config-viewer-path-text').textContent = hooksStatusCache[agent].config_path;
                        document.getElementById('config-viewer-path-bar').style.display = 'flex';
                    } else {
                        document.getElementById('config-viewer-path-text').textContent = '';
                        document.getElementById('config-viewer-path-bar').style.display = 'none';
                    }
                    configViewerCode.innerHTML = syntaxHighlightJSON(content || "{}");
                    configViewerModal.style.display = 'flex';
                } catch (e) {
                    console.error(e);
                }
            });
        }
        
        if (btnInject) {
            btnInject.addEventListener('click', async () => {
                try {
                    const diff = await invoke('preview_inject_hook', { agent });
                    renderDiff(diff.before, diff.after, injectBeforeCode, injectAfterCode);
                    
                    if (hooksStatusCache && hooksStatusCache[agent] && hooksStatusCache[agent].config_path) {
                        document.getElementById('inject-path-text').textContent = hooksStatusCache[agent].config_path;
                        document.getElementById('inject-path-bar').style.display = 'flex';
                    } else {
                        document.getElementById('inject-path-bar').style.display = 'none';
                    }
                    
                    currentInjectingAgent = agent;
                    hookModal.style.display = 'flex';
                } catch(e) {
                    alert("Preview failed: " + e);
                }
            });
        }
        
        if (btnUninstall) {
            btnUninstall.addEventListener('click', async () => {
                try {
                    const diff = await invoke('preview_uninstall_hook', { agent });
                    renderDiff(diff.before, diff.after, uninstallBeforeCode, uninstallAfterCode);
                    
                    if (hooksStatusCache && hooksStatusCache[agent] && hooksStatusCache[agent].config_path) {
                        document.getElementById('uninstall-path-text').textContent = hooksStatusCache[agent].config_path;
                        document.getElementById('uninstall-path-bar').style.display = 'flex';
                    } else {
                        document.getElementById('uninstall-path-bar').style.display = 'none';
                    }
                    
                    currentUninstallingAgent = agent;
                    uninstallModal.style.display = 'flex';
                } catch(e) {
                    alert("Preview uninstall failed: " + e);
                }
            });
        }
    });

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
