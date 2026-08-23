import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { PixelSpriteRenderer } from "./pet/PixelSpriteRenderer.js";
import { applyTranslations, t } from "./i18n.js";

let renderer = null;
let currentLang = 'en-US';
let currentRuntimeMode = 'local';
let remoteConnectionStatus = 'offline';
let petHoverHideEnabled = false;
window.celebrationMs = 4000; // default, updated from config

function setPetHoverHideEnabled(enabled) {
    petHoverHideEnabled = enabled === true;
    if (!petHoverHideEnabled) {
        document.getElementById('unified-container')?.classList.remove('hover-hidden');
    }
}

function setupPetHoverHide() {
    const windowSurface = document.getElementById('unified-container');
    if (!windowSurface || windowSurface.dataset.hoverHideBound === 'true') return;

    windowSurface.dataset.hoverHideBound = 'true';
    windowSurface.addEventListener('pointerenter', () => {
        if (petHoverHideEnabled) windowSurface.classList.add('hover-hidden');
    });
    windowSurface.addEventListener('pointerleave', () => {
        windowSurface.classList.remove('hover-hidden');
    });
}

function updateRuntimeModeBadge() {
    const bar = document.getElementById('runtime-mode-bar');
    const badge = document.getElementById('runtime-mode-badge');
    if (!bar || !badge) return;
    if (currentRuntimeMode !== 'remote') {
        bar.style.display = 'none';
        return;
    }
    const statusKeys = {
        connecting: 'runtime_status_connecting',
        connected: 'runtime_status_connected',
        reconnecting: 'runtime_status_reconnecting',
        authentication_failed: 'runtime_status_authentication_failed',
        incompatible_protocol: 'runtime_status_incompatible_protocol',
        offline: 'runtime_status_offline',
    };
    const statusKey = statusKeys[remoteConnectionStatus] || 'runtime_status_offline';
    badge.dataset.status = remoteConnectionStatus;
    badge.title = `${t('runtime_mode_remote', currentLang)} · ${t(statusKey, currentLang)}`;
    badge.setAttribute('aria-label', badge.title);
    bar.style.display = 'flex';
}

async function fetchManifest(spriteName) {
    try {
        let res = await fetch(`/sprites/${spriteName}/pack.json`);
        if (!res.ok) {
            res = await fetch(`/sprites/${spriteName}/manifest.json`);
        }
        return await res.json();
    } catch (e) {
        console.error("fetchManifest failed", e);
        throw e;
    }
}

function setupContextMenu() {
    const contextMenu = document.getElementById("context-menu");
    const menuSettings = document.getElementById("menu-settings");

    // Custom Context Menu
    document.addEventListener('contextmenu', e => {
        e.preventDefault();
        // Position menu at mouse coordinates
        contextMenu.style.display = "block";
        
        // Ensure menu doesn't go off screen
        const menuWidth = contextMenu.offsetWidth || 150;
        const menuHeight = contextMenu.offsetHeight || 40;
        
        let x = e.clientX;
        let y = e.clientY;
        
        if (x + menuWidth > window.innerWidth) x = window.innerWidth - menuWidth;
        if (y + menuHeight > window.innerHeight) y = window.innerHeight - menuHeight;
        
        contextMenu.style.left = `${x}px`;
        contextMenu.style.top = `${y}px`;
    });

    document.addEventListener('click', e => {
        if (e.target !== contextMenu && !contextMenu.contains(e.target)) {
            contextMenu.style.display = "none";
        }
    });

    const menuQuit = document.getElementById("menu-quit");

    // Settings Modal Open
    menuSettings.addEventListener("click", async () => {
        contextMenu.style.display = "none";
        try {
            await invoke("open_settings_window");
        } catch (e) {
            console.error("Failed to open settings window:", e);
        }
    });

    // Quit Application
    if (menuQuit) {
        menuQuit.addEventListener("click", async () => {
            contextMenu.style.display = "none";
            try {
                await invoke("quit_app");
            } catch (e) {
                console.error("Failed to quit app:", e);
            }
        });
    }
}

let currentSpriteId = null;

async function loadActiveSpritePack() {
    try {
        const packInfo = await invoke("get_active_sprite_pack");
        if (packInfo && packInfo.manifest) {
            currentSpriteId = packInfo.manifest.id;
            await renderer.loadSpritePack(packInfo);
            renderer.playAnimation("idle");
        }
    } catch (e) {
        console.error("Failed to load active sprite pack via command, trying fallback", e);
        try {
            const manifest = await fetchManifest("british-blue");
            await renderer.loadSpritePack(manifest);
            renderer.playAnimation("idle");
        } catch (err) {
            console.error("Fallback load failed", err);
        }
    }
}

async function init() {
    const container = document.getElementById("pet-container");
    
    // Dragging is now handled specifically by PixelSpriteRenderer, BubbleOverlay, and stats.js

    setupContextMenu();

    renderer = new PixelSpriteRenderer();
    renderer.init(container);
    setupPetHoverHide();

    try {
        const config = await invoke("get_config");
        await applyConfigToWindow(config);
    } catch (e) {
        console.error("Failed to load initial config", e);
        document.body.style.opacity = '1';
    }

    await loadActiveSpritePack();

    // Listen for state changes from Rust Backend
    await listen("state_changed", (event) => {
        const state = event.payload;
        if (!state) return;

        if (currentRuntimeMode === 'remote' && remoteConnectionStatus !== 'connected') {
            remoteConnectionStatus = 'connected';
            updateRuntimeModeBadge();
        }

        // Clear any pending idle fallback
        if (window.idleFallbackTimer) {
            clearTimeout(window.idleFallbackTimer);
            window.idleFallbackTimer = null;
        }

        // Map mood to animation
        switch (state.mood) {
            case "Thinking":
                renderer.playAnimation("thinking");
                break;
            case "Busy":
                renderer.playAnimation("working");
                break;
            case "Interacting":
            case "Happy":
                renderer.playAnimation("interacting");
                break;
            case "Celebrating":
                renderer.playAnimation("celebrating");
                // Automatically revert to idle after celebration
                window.idleFallbackTimer = setTimeout(() => {
                    renderer.playAnimation("idle");
                }, window.celebrationMs);
                break;
            case "Watching":
                renderer.playAnimation("watching");
                break;
            case "Alarmed":
                renderer.playAnimation("alarmed");
                break;
            case "Sleepy":
                renderer.playAnimation("sleeping");
                break;
            default:
                renderer.playAnimation("idle");
        }
    });

    await listen("connection_status_changed", (event) => {
        remoteConnectionStatus = event.payload || 'offline';
        updateRuntimeModeBadge();
    });

    // Listen for config changes from Rust Backend
    await listen("config_changed", (event) => {
        applyConfigToWindow(event.payload);
    });
}

let lastWidth = 0;
let lastHeight = 0;

function updateWindowSize() {
    const container = document.getElementById('unified-container');
    if (!container) return;
    const rect = container.getBoundingClientRect();
    const width = Math.ceil(rect.width);
    const height = Math.ceil(rect.height);

    if (width === 0 || height === 0) return;
    if (width === lastWidth && height === lastHeight) return;

    const anchorBottom = lastHeight > 0;
    lastWidth = width;
    lastHeight = height;

    // Resize and reposition in the desktop layer as one anchored operation.
    // Keeping the bottom edge fixed makes bubble growth consume space above
    // the pet instead of pushing the pet down.
    invoke('resize_main_window', { width, height, anchorBottom }).catch((e) => {
        console.error("Failed to resize window dynamically:", e);
    });
}

window.requestDashboardResize = updateWindowSize;

async function applyConfigToWindow(config) {
    if (!config) return;

    currentRuntimeMode = config.runtime?.mode || 'local';
    if (currentRuntimeMode === 'remote' && remoteConnectionStatus === 'offline') {
        remoteConnectionStatus = 'connecting';
    }
    updateRuntimeModeBadge();

    if (config.general && config.general.language) {
        currentLang = config.general.language;
        applyTranslations(currentLang);
    }

    if (!config.renderer || !config.renderer['desktop-pet']) {
        setPetHoverHideEnabled(false);
        document.body.style.opacity = '1';
        return;
    }
    
    const petConf = config.renderer['desktop-pet'];
    
    if (petConf.sprite && currentSpriteId && petConf.sprite !== currentSpriteId) {
        loadActiveSpritePack();
    }

    document.body.style.opacity = petConf.opacity !== undefined ? petConf.opacity : 1;
    
    if (petConf.scale !== undefined && renderer) {
        window.petScale = petConf.scale;
        
        const container = document.getElementById('unified-container');
        if (container) {
            container.style.transform = `scale(${window.petScale})`;
        }
    }
    
    // Apply display toggles
    const bubbleContainer = document.getElementById('bubble-container');
    if (bubbleContainer) {
        bubbleContainer.style.display = (petConf.show_task_bubble !== false) ? 'flex' : 'none';
    }
    
    const petContainerUI = document.getElementById('pet-container');
    if (petContainerUI) {
        petContainerUI.style.display = (petConf.show_pet !== false) ? 'flex' : 'none';
    }
    setPetHoverHideEnabled(petConf.hide_on_hover === true);
    
    const frameContainer = document.getElementById('unified-container');
    if (frameContainer) {
        frameContainer.classList.toggle('show-frame', petConf.show_window_frame === true);
    }

    const statsContainer = document.getElementById('stats-container');
    if (statsContainer) {
        window.setDashboardStyle?.(petConf.dashboard_style || 'classic');
        window.setDashboardLayout?.(petConf.dashboard_layout || 'vertical');
        const dashboardPosition = ['left', 'bottom', 'right'].includes(petConf.dashboard_position)
            ? petConf.dashboard_position
            : 'bottom';
        const petStage = document.getElementById('pet-stage');
        if (petStage) {
            petStage.dataset.dashboardPosition = dashboardPosition;
            petStage.dataset.dashboardLayout = ['horizontal', 'vertical'].includes(petConf.dashboard_layout)
                ? petConf.dashboard_layout
                : 'vertical';
            petStage.dataset.dashboardAlignment = ['top', 'center', 'bottom'].includes(petConf.dashboard_alignment)
                ? petConf.dashboard_alignment
                : 'bottom';
        }
        const isShow = petConf.show_dashboard !== false;
        statsContainer.style.display = isShow ? 'block' : 'none';
        if (isShow) {
            window.startStatsPolling?.();
        } else {
            window.stopStatsPolling?.();
        }
    }

    if (petConf.celebration_secs !== undefined) {
        window.celebrationMs = petConf.celebration_secs * 1000;
    }
    
    // Force window size update after changing display states
    updateWindowSize();
    
    // Setup observer if not already done
    const unifiedContainer = document.getElementById('unified-container');
    if (unifiedContainer && !window.containerObserver) {
        window.containerObserver = new ResizeObserver(() => {
            updateWindowSize();
        });
        window.containerObserver.observe(unifiedContainer);
    }
}

window.addEventListener("DOMContentLoaded", () => {
    init();
});
