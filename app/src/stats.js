import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

let statsInterval = null;
let isFetching = false;
const DASHBOARD_STYLES = new Set(['classic', 'minimal']);
const DASHBOARD_LAYOUTS = new Set(['horizontal', 'vertical']);

export function setDashboardStyle(style) {
    const statsContainer = document.getElementById('stats-container');
    if (!statsContainer) return;
    statsContainer.dataset.dashboardStyle = DASHBOARD_STYLES.has(style) ? style : 'classic';
    requestAnimationFrame(() => window.requestDashboardResize?.());
}

export function setDashboardLayout(layout) {
    const statsContainer = document.getElementById('stats-container');
    if (!statsContainer) return;
    statsContainer.dataset.dashboardLayout = DASHBOARD_LAYOUTS.has(layout) ? layout : 'vertical';
    requestAnimationFrame(() => window.requestDashboardResize?.());
}

function updateUsageState(element, percent) {
    const row = element?.closest('.stat-row');
    if (!row) return;
    row.classList.toggle('is-warning', percent >= 70 && percent < 90);
    row.classList.toggle('is-critical', percent >= 90);
}

export async function updateStats() {
    if (isFetching) return;
    const statsContainer = document.getElementById('stats-container');
    if (statsContainer && statsContainer.style.display === 'none') {
        stopStatsPolling();
        return;
    }

    isFetching = true;
    try {
        const stats = await invoke("get_system_stats");
        
        // CPU
        const cpuPercent = Math.min(100, Math.max(0, stats.cpu_usage)).toFixed(1);
        const barCpu = document.getElementById('bar-cpu');
        const valCpu = document.getElementById('val-cpu');
        if (barCpu) barCpu.style.width = `${cpuPercent}%`;
        if (valCpu) valCpu.innerText = `${Math.round(cpuPercent)}%`;
        updateUsageState(valCpu, Number(cpuPercent));

        // RAM
        const ramPercent = stats.memory_total > 0 ? ((stats.memory_used / stats.memory_total) * 100).toFixed(1) : 0;
        const barRam = document.getElementById('bar-ram');
        const valRam = document.getElementById('val-ram');
        if (barRam) barRam.style.width = `${ramPercent}%`;
        if (valRam) valRam.innerText = `${Math.round(ramPercent)}%`;
        updateUsageState(valRam, Number(ramPercent));

        // Disk
        const diskPercent = stats.disk_total > 0 ? ((stats.disk_used / stats.disk_total) * 100).toFixed(1) : 0;
        const barDisk = document.getElementById('bar-disk');
        const valDisk = document.getElementById('val-disk');
        if (barDisk) barDisk.style.width = `${diskPercent}%`;
        if (valDisk) valDisk.innerText = `${Math.round(diskPercent)}%`;
        updateUsageState(valDisk, Number(diskPercent));
    } catch (e) {
        console.error("Failed to fetch stats:", e);
    } finally {
        isFetching = false;
    }
}

export function startStatsPolling() {
    if (statsInterval) return;
    updateStats();
    statsInterval = setInterval(updateStats, 2000);
}

export function stopStatsPolling() {
    if (statsInterval) {
        clearInterval(statsInterval);
        statsInterval = null;
    }
}

if (typeof window !== "undefined") {
    window.startStatsPolling = startStatsPolling;
    window.stopStatsPolling = stopStatsPolling;
    window.setDashboardStyle = setDashboardStyle;
    window.setDashboardLayout = setDashboardLayout;

    window.addEventListener("DOMContentLoaded", async () => {
        // Make stats drag the main window
        const statsContainer = document.getElementById('stats-container');
        if (statsContainer) {
            try {
                const config = await invoke('get_config');
                const petConfig = config?.renderer?.['desktop-pet'];
                setDashboardStyle(petConfig?.dashboard_style || 'classic');
                setDashboardLayout(petConfig?.dashboard_layout || 'vertical');
            } catch (e) {
                console.error('Failed to load dashboard appearance:', e);
                setDashboardStyle('classic');
                setDashboardLayout('vertical');
            }

            listen('config_changed', (event) => {
                const petConfig = event.payload?.renderer?.['desktop-pet'];
                setDashboardStyle(petConfig?.dashboard_style || 'classic');
                setDashboardLayout(petConfig?.dashboard_layout || 'vertical');
            }).catch(console.error);

            statsContainer.addEventListener('mousedown', (e) => {
                if (e.button === 0) {
                    invoke('drag_main_window').catch(console.error);
                }
            });

            if (statsContainer.style.display !== 'none') {
                startStatsPolling();
            }
        }
    });

    window.addEventListener("beforeunload", () => {
        stopStatsPolling();
    });
}
