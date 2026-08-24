import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { PixelSpriteRenderer } from "./pet/PixelSpriteRenderer.js";
import { applyTranslations, t } from "./i18n.js";

let renderer = null;
let currentLang = 'en-US';
let currentRuntimeMode = 'local';
let remoteConnectionStatus = 'offline';
let clickThroughEnabled = false;
window.clickThroughEnabled = false;
// While the context menu is open the window is temporarily grown to fit it;
// updateWindowSize must not fight that resize until the menu closes.
let contextMenuOpen = false;
window.celebrationMs = 4000; // default, updated from config

// Hover transparency: while the pointer rests over the pet we dim the content
// so what's behind is not blocked. `hoverOpacity === null` disables the effect.
// Hover is detected by a native tracking area (see the Rust `hover` module),
// which emits `pet_hover_changed`; a non-activating panel doesn't deliver
// mouseover to the webview until clicked, so DOM mouseenter alone is unreliable.
let baseOpacity = 1;
let hoverOpacity = null;
let hoverHideEnabled = true;
let isHovering = false;

function getOpacityTarget() {
    // Dim the content container (pet + bubble + dashboard) rather than <body>
    // so the right-click context menu — a sibling of #unified-container — stays
    // fully opaque.
    return document.getElementById('unified-container') || document.body;
}

function applyOpacity() {
    const target = (isHovering && hoverHideEnabled && hoverOpacity !== null)
        ? hoverOpacity
        : baseOpacity;
    getOpacityTarget().style.opacity = String(target);
}

async function setupHoverTransparency() {
    getOpacityTarget().style.transition = 'opacity 0.15s ease';
    // System-wide mouse-move monitors (Rust) report hover regardless of whether
    // the non-activating panel is focused, so this fires the moment the pointer
    // reaches the pet — no click needed.
    await listen('pet_hover_changed', (event) => {
        isHovering = !!event.payload;
        applyOpacity();
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
    const menuClickThrough = document.getElementById("menu-click-through");

    // Show/hide the menu and, while it is open, suspend left-click pass-through
    // so the menu items stay clickable; restore the configured pass-through on
    // close.
    //
    // The menu must never leave the screen: query the window's screen rect and
    // the monitor work area, clamp the menu origin into the work area, then
    // grow the (transparent) window to the union of both rects. Growing left/
    // up moves the window origin, so the pet container is offset by the same
    // delta — the pet itself never moves.
    let menuFrameRestore = null;

    async function positionMenuAt(clientX, clientY) {
        const menuWidth = contextMenu.offsetWidth || 150;
        const menuHeight = contextMenu.offsetHeight || 40;

        let geom;
        try {
            geom = await invoke("get_menu_geometry");
        } catch (e) {
            console.error("Failed to get window geometry:", e);
            contextMenu.style.left = `${Math.max(0, clientX)}px`;
            contextMenu.style.top = `${Math.max(0, clientY)}px`;
            return;
        }

        // Menu origin at the cursor, clamped into the monitor work area.
        let mx = Math.min(geom.x + clientX, geom.work_right - menuWidth);
        let my = Math.min(geom.y + clientY, geom.work_bottom - menuHeight);
        mx = Math.max(mx, geom.work_left);
        my = Math.max(my, geom.work_top);

        // Window frame = union of the current rect and the menu rect.
        const nx = Math.min(geom.x, mx);
        const ny = Math.min(geom.y, my);
        const right = Math.max(geom.x + geom.width, mx + menuWidth);
        const bottom = Math.max(geom.y + geom.height, my + menuHeight);

        menuFrameRestore = { x: geom.x, y: geom.y, width: geom.width, height: geom.height };
        const container = document.getElementById("unified-container");
        if (container) {
            container.style.marginLeft = `${geom.x - nx}px`;
            container.style.marginTop = `${geom.y - ny}px`;
        }
        try {
            await invoke("set_main_window_frame", {
                x: nx, y: ny, width: right - nx, height: bottom - ny,
            });
        } catch (e) {
            console.error("Failed to grow window for menu:", e);
        }

        contextMenu.style.left = `${mx - nx}px`;
        contextMenu.style.top = `${my - ny}px`;
    }

    function restoreWindowFrame() {
        const container = document.getElementById("unified-container");
        if (container) {
            container.style.marginLeft = "";
            container.style.marginTop = "";
        }
        if (menuFrameRestore) {
            const frame = menuFrameRestore;
            menuFrameRestore = null;
            invoke("set_main_window_frame", frame)
                .catch((e) => console.error("Failed to restore window frame:", e));
        }
        // Reconcile if the content size changed while the menu was open.
        updateWindowSize();
    }

    function setMenuOpen(open) {
        contextMenuOpen = open;
        contextMenu.style.display = open ? "block" : "none";
        invoke("set_click_through_enabled", { enabled: open ? false : clickThroughEnabled })
            .catch((e) => console.error("Failed to toggle click-through:", e));
        if (open) {
            armMenuIdleTimer();
        } else {
            disarmMenuIdleTimer();
            restoreWindowFrame();
        }
    }

    // Unlike native menus, our HTML menu is not dismissed by clicks that land
    // outside the window (a non-activating panel never sees them), so it would
    // linger on screen. Auto-close it after a short idle period instead; while
    // the pointer rests on the menu the timeout is suspended.
    const MENU_IDLE_MS = 5000;
    let menuIdleTimer = null;
    let lastMenuActivity = 0;

    document.addEventListener("mousemove", () => {
        if (contextMenuOpen) lastMenuActivity = Date.now();
    });

    function armMenuIdleTimer() {
        lastMenuActivity = Date.now();
        if (menuIdleTimer) clearInterval(menuIdleTimer);
        menuIdleTimer = setInterval(() => {
            if (contextMenu.matches(":hover")) return;
            if (Date.now() - lastMenuActivity >= MENU_IDLE_MS) setMenuOpen(false);
        }, 1000);
    }

    function disarmMenuIdleTimer() {
        if (menuIdleTimer) {
            clearInterval(menuIdleTimer);
            menuIdleTimer = null;
        }
    }

    // Open the menu at window-relative CSS coordinates. Shared by the webview's
    // own contextmenu event (pass-through off) and the Rust right-click monitor
    // (pass-through on, when the webview never sees the click because the
    // window is hit-test transparent).
    function openContextMenuAt(clientX, clientY) {
        setMenuOpen(true);
        if (menuClickThrough) {
            menuClickThrough.classList.toggle('checked', clickThroughEnabled);
        }
        positionMenuAt(clientX, clientY);
    }

    // Custom Context Menu
    document.addEventListener('contextmenu', e => {
        e.preventDefault();
        openContextMenuAt(e.clientX, e.clientY);
    });

    // Right-click captured by the Rust monitor while pass-through is on.
    listen('pet_context_menu', (event) => {
        openContextMenuAt(event.payload.x, event.payload.y);
    }).catch((e) => console.error("Failed to listen for pet_context_menu:", e));

    document.addEventListener('click', e => {
        if (e.target !== contextMenu && !contextMenu.contains(e.target)) {
            setMenuOpen(false);
        }
    });

    const menuQuit = document.getElementById("menu-quit");

    // Settings Modal Open
    menuSettings.addEventListener("click", async () => {
        setMenuOpen(false);
        try {
            await invoke("open_settings_window");
        } catch (e) {
            console.error("Failed to open settings window:", e);
        }
    });

    // Toggle click-through without opening the settings window.
    if (menuClickThrough) {
        menuClickThrough.addEventListener("click", async () => {
            const next = !clickThroughEnabled;
            try {
                const config = await invoke("get_config");
                config.renderer['desktop-pet'].click_through = next;
                await invoke("save_config", { config });
                clickThroughEnabled = next;
                window.clickThroughEnabled = next;
                menuClickThrough.classList.toggle('checked', next);
            } catch (e) {
                console.error("Failed to toggle click-through:", e);
            }
            setMenuOpen(false);
        });
    }

    // Quit Application
    if (menuQuit) {
        menuQuit.addEventListener("click", async () => {
            setMenuOpen(false);
            try {
                await invoke("quit_app");
            } catch (e) {
                console.error("Failed to quit app:", e);
            }
        });
    }
}

let currentSpriteId = null;
let currentSpriteRevision = null;

async function loadActiveSpritePack(force = true) {
    try {
        const packInfo = await invoke("get_active_sprite_pack");
        if (packInfo && packInfo.manifest) {
            const samePack = currentSpriteId === packInfo.manifest.id
                && currentSpriteRevision === packInfo.asset_revision;
            if (!force && samePack) return;

            currentSpriteId = packInfo.manifest.id;
            currentSpriteRevision = packInfo.asset_revision;
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
    await setupHoverTransparency();

    renderer = new PixelSpriteRenderer();
    renderer.init(container);

    try {
        const config = await invoke("get_config");
        await applyConfigToWindow(config);
    } catch (e) {
        console.error("Failed to load initial config", e);
        getOpacityTarget().style.opacity = '1';
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
    await listen("config_changed", async (event) => {
        await applyConfigToWindow(event.payload);
    });

    await listen("sprite_pack_imported", async (event) => {
        const imported = event.payload;
        if (!imported || !imported.manifest || imported.manifest.id !== currentSpriteId) return;

        currentSpriteRevision = imported.asset_revision;
        await renderer.loadSpritePack(imported);
        renderer.playAnimation("idle");
    });
}

let lastWidth = 0;
let lastHeight = 0;

function updateWindowSize() {
    // The context menu temporarily grows the window; don't shrink it back
    // mid-menu.
    if (contextMenuOpen) return;
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
        baseOpacity = 1;
        hoverOpacity = null;
        applyOpacity();
        return;
    }
    
    const petConf = config.renderer['desktop-pet'];
    
    if (petConf.sprite && currentSpriteId) {
        await loadActiveSpritePack(false);
    }

    baseOpacity = petConf.opacity !== undefined ? petConf.opacity : 1;
    // Only dim on hover when it is actually more transparent than the base.
    hoverOpacity = (petConf.hover_opacity !== undefined && petConf.hover_opacity < baseOpacity)
        ? petConf.hover_opacity
        : null;
    hoverHideEnabled = petConf.hide_on_hover === true;
    clickThroughEnabled = petConf.click_through === true;
    window.clickThroughEnabled = clickThroughEnabled;
    applyOpacity();
    
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
