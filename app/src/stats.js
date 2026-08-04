import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

let statsInterval = null;
let isFetching = false;

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

        // RAM
        const ramPercent = stats.memory_total > 0 ? ((stats.memory_used / stats.memory_total) * 100).toFixed(1) : 0;
        const barRam = document.getElementById('bar-ram');
        const valRam = document.getElementById('val-ram');
        if (barRam) barRam.style.width = `${ramPercent}%`;
        if (valRam) valRam.innerText = `${Math.round(ramPercent)}%`;

        // Disk
        const diskPercent = stats.disk_total > 0 ? ((stats.disk_used / stats.disk_total) * 100).toFixed(1) : 0;
        const barDisk = document.getElementById('bar-disk');
        const valDisk = document.getElementById('val-disk');
        if (barDisk) barDisk.style.width = `${diskPercent}%`;
        if (valDisk) valDisk.innerText = `${Math.round(diskPercent)}%`;
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

    window.addEventListener("DOMContentLoaded", () => {
        // Make stats drag the main window
        const statsContainer = document.getElementById('stats-container');
        if (statsContainer) {
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
