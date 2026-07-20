export const translations = {
    'zh-CN': {
        'nav_close': '关闭设置',
        'nav_search': '搜索设置...',
        'nav_pref': '偏好',
        'nav_general': '常规配置',
        'nav_pet': '桌面宠物配置',
        'nav_sys': '系统',
        'nav_hooks': 'Hooks',
        'nav_about': '关于',
        'title_general': '常规配置',
        'title_pet': '桌面宠物配置',
        'title_hooks': 'Hooks',
        'title_about': '关于',
        'group_general': '通用设置',
        'lbl_lang': '应用语言',
        'desc_lang': '设置 Familiar 客户端的显示语言',
        'lbl_autostart': '开机自启',
        'desc_autostart': '系统启动时自动运行 Familiar 守护进程',
        'lbl_api_port': 'API 端口',
        'desc_api_port': '外部服务访问 Familiar HTTP API 的本地端口',
        'group_pet': '桌宠外观与行为',
        'lbl_pet_scale': '桌宠缩放比例',
        'desc_pet_scale': '调整桌面宠物的显示大小 (1.0 - 5.0)',
        'lbl_bubble_scale': '气泡缩放比例',
        'desc_bubble_scale': '调整信息气泡的显示大小 (0.5 - 3.0)',
        'lbl_always_top': '总是在最前',
        'desc_always_top': '让桌宠悬浮在所有系统窗口之上，避免被遮挡',
        'lbl_opacity': '桌宠不透明度',
        'desc_opacity': '调整桌宠的基础透明度 (0.1 - 1.0)',
        'group_ipc': 'Agent 进程间通信 (IPC)',
        'lbl_uds': 'Unix Domain Socket 路径',
        'desc_uds': '用于与本地原生 Agent 工具 (如 Antigravity, Claude Code) 进行极速通信的 UDS 文件路径。<br><span style="color:var(--text-muted);font-size:11px;">通常无需修改，保持默认即可。</span>',
        'lbl_tcp': '备用 TCP 端口',
        'desc_tcp': '当操作系统不支持 UDS 时（如 Windows），使用的备用 TCP 通信端口',
        'group_familiar': 'Familiar',
        'lbl_version': '当前版本',
        'desc_version': 'v1.0.0 (Beta) - Powered by Rust & Tauri 2.0',
        'lbl_license': '开源协议',
        'desc_license': 'MIT License',
        'lbl_url': '项目地址',
        'btn_save': '保存并应用',
        'msg_saving': '保存中...',
        'msg_saved': '设置已保存！部分设置重启生效。',
        'msg_failed': '保存失败: ',
        'menu_settings': '⚙️ 设置',
        'status_waiting': '等待任务中...',
    },
    'en-US': {
        'nav_close': 'Close Settings',
        'nav_search': 'Search settings...',
        'nav_pref': 'Preferences',
        'nav_general': 'General',
        'nav_pet': 'Desktop Pet',
        'nav_sys': 'System',
        'nav_hooks': 'Hooks',
        'nav_about': 'About',
        'title_general': 'General Configuration',
        'title_pet': 'Desktop Pet Configuration',
        'title_hooks': 'Hooks',
        'title_about': 'About',
        'group_general': 'General Settings',
        'lbl_lang': 'Language',
        'desc_lang': 'Set the display language of the Familiar client',
        'lbl_autostart': 'Auto Start',
        'desc_autostart': 'Run Familiar daemon automatically on system startup',
        'lbl_api_port': 'API Port',
        'desc_api_port': 'Local port for external services to access the HTTP API',
        'group_pet': 'Appearance & Behavior',
        'lbl_pet_scale': 'Pet Scale',
        'desc_pet_scale': 'Adjust the display size of the desktop pet (1.0 - 5.0)',
        'lbl_bubble_scale': 'Bubble Scale',
        'desc_bubble_scale': 'Adjust the display size of the info bubble (0.5 - 3.0)',
        'lbl_always_top': 'Always on Top',
        'desc_always_top': 'Keep the pet floating above all system windows',
        'lbl_opacity': 'Pet Opacity',
        'desc_opacity': 'Adjust the base opacity of the desktop pet (0.1 - 1.0)',
        'group_ipc': 'Agent IPC',
        'lbl_uds': 'Unix Domain Socket Path',
        'desc_uds': 'UDS file path for ultra-fast communication with local native Agent tools.<br><span style="color:var(--text-muted);font-size:11px;">Usually no need to modify, keep default.</span>',
        'lbl_tcp': 'Fallback TCP Port',
        'desc_tcp': 'Fallback TCP port when the OS does not support UDS (e.g., Windows)',
        'group_familiar': 'Familiar',
        'lbl_version': 'Current Version',
        'desc_version': 'v1.0.0 (Beta) - Powered by Rust & Tauri 2.0',
        'lbl_license': 'Open Source License',
        'desc_license': 'MIT License',
        'lbl_url': 'Project URL',
        'btn_save': 'Save and Apply',
        'msg_saving': 'Saving...',
        'msg_saved': 'Settings saved! Some take effect after restart.',
        'msg_failed': 'Save failed: ',
        'menu_settings': '⚙️ Settings',
        'status_waiting': 'Waiting for task...',
    }
};

export function applyTranslations(lang) {
    const dict = translations[lang] || translations['en-US'];
    document.querySelectorAll('[data-i18n]').forEach(el => {
        const key = el.getAttribute('data-i18n');
        if (dict[key]) {
            if (el.tagName === 'INPUT' && el.type === 'text' && el.placeholder) {
                el.placeholder = dict[key];
            } else {
                // If it contains child elements that we want to preserve, we might need a more complex update,
                // but setting innerHTML is the easiest for now, especially with the <br> and <span> in descriptions.
                // However, for buttons with SVGs inside, we need to be careful!
                if (el.hasAttribute('data-i18n-html')) {
                    el.innerHTML = dict[key];
                } else if (el.hasAttribute('data-i18n-text-only')) {
                    // Update only text nodes to preserve SVGs
                    for (let child of el.childNodes) {
                        if (child.nodeType === Node.TEXT_NODE && child.textContent.trim().length > 0) {
                            child.textContent = ' ' + dict[key];
                            break;
                        }
                    }
                } else {
                    el.innerHTML = dict[key];
                }
            }
        }
    });
}

export function t(key, lang) {
    const dict = translations[lang] || translations['en-US'];
    return dict[key] || key;
}
