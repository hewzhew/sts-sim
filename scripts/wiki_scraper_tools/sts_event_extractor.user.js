// ==UserScript==
// @name         Slay the Spire Wiki Event Extractor
// @namespace    https://github.com/sts-sim
// @version      1.0
// @description  提取杀戮尖塔 Wiki 事件数据，保存为 JSON
// @author       STS-Sim
// @match        https://slaythespire.wiki.gg/wiki/*
// @match        https://slay-the-spire.fandom.com/wiki/*
// @icon         https://slaythespire.wiki.gg/favicon.ico
// @grant        GM_setClipboard
// @grant        GM_notification
// @grant        GM_setValue
// @grant        GM_getValue
// @grant        GM_addStyle
// ==/UserScript==

(function() {
    'use strict';

    // ===================== 配置 =====================
    const EVENT_PAGES = [
        // Shrines (All Acts)
        "A_Note_For_Yourself", "Bonfire_Spirits", "Duplicator", "Golden_Shrine",
        "Lab", "Match_and_Keep", "Ominous_Forge", "Purifier", "The_Divine_Fountain",
        "The_Woman_in_Blue", "Transmogrifier", "Upgrade_Shrine", "We_Meet_Again!",
        "Wheel_of_Change",
        // Act 1
        "Big_Fish", "Dead_Adventurer", "Face_Trader", "Golden_Idol_(Event)",
        "Hypnotizing_Colored_Mushrooms", "Living_Wall", "Scrap_Ooze", "Shining_Light",
        "The_Cleric", "The_Ssssserpent", "Wing_Statue", "World_of_Goop",
        // Act 2
        "Ancient_Writing", "Augmenter", "Council_of_Ghosts", "Cursed_Tome",
        "Designer_In-Spire", "Forgotten_Altar", "Knowing_Skull", "Masked_Bandits",
        "N%27loth", "Old_Beggar", "Pleading_Vagrant", "The_Colosseum", "The_Joust",
        "The_Library", "The_Mausoleum", "The_Nest", "Vampires",
        // Act 3
        "Falling", "Mind_Bloom", "Mysterious_Sphere", "Secret_Portal",
        "Sensory_Stone", "The_Moai_Head", "Tomb_of_Lord_Red_Mask", "Winding_Halls"
    ];

    // ===================== 样式 =====================
    GM_addStyle(`
        #sts-extractor-panel {
            position: fixed;
            top: 10px;
            right: 10px;
            z-index: 99999;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            border: 2px solid #e94560;
            border-radius: 12px;
            padding: 15px;
            font-family: 'Segoe UI', Arial, sans-serif;
            color: #eee;
            min-width: 280px;
            box-shadow: 0 8px 32px rgba(233, 69, 96, 0.3);
        }
        #sts-extractor-panel h3 {
            margin: 0 0 12px 0;
            color: #e94560;
            font-size: 16px;
            display: flex;
            align-items: center;
            gap: 8px;
        }
        #sts-extractor-panel .btn {
            display: block;
            width: 100%;
            padding: 10px 15px;
            margin: 8px 0;
            border: none;
            border-radius: 6px;
            cursor: pointer;
            font-size: 14px;
            font-weight: 600;
            transition: all 0.2s;
        }
        #sts-extractor-panel .btn-primary {
            background: linear-gradient(135deg, #e94560 0%, #ff6b6b 100%);
            color: white;
        }
        #sts-extractor-panel .btn-primary:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 15px rgba(233, 69, 96, 0.4);
        }
        #sts-extractor-panel .btn-secondary {
            background: #2a2a4a;
            color: #aaa;
            border: 1px solid #444;
        }
        #sts-extractor-panel .btn-secondary:hover {
            background: #3a3a5a;
            color: #fff;
        }
        #sts-extractor-panel .status {
            font-size: 12px;
            color: #888;
            margin-top: 10px;
            padding: 8px;
            background: rgba(0,0,0,0.3);
            border-radius: 4px;
        }
        #sts-extractor-panel .status.success { color: #4ade80; }
        #sts-extractor-panel .status.error { color: #f87171; }
        #sts-extractor-panel .progress-bar {
            height: 4px;
            background: #333;
            border-radius: 2px;
            margin-top: 8px;
            overflow: hidden;
        }
        #sts-extractor-panel .progress-bar .fill {
            height: 100%;
            background: linear-gradient(90deg, #e94560, #ff6b6b);
            transition: width 0.3s;
        }
        #sts-extractor-panel .info {
            font-size: 11px;
            color: #666;
            margin-top: 5px;
        }
        #sts-extractor-panel .toggle-btn {
            position: absolute;
            top: -10px;
            right: -10px;
            width: 24px;
            height: 24px;
            border-radius: 50%;
            background: #e94560;
            border: none;
            color: white;
            cursor: pointer;
            font-size: 14px;
        }
        #sts-extractor-panel.minimized {
            min-width: auto;
            padding: 8px 12px;
        }
        #sts-extractor-panel.minimized > *:not(h3):not(.toggle-btn) {
            display: none;
        }
        .extracted-preview {
            max-height: 150px;
            overflow-y: auto;
            font-size: 11px;
            background: rgba(0,0,0,0.2);
            padding: 8px;
            border-radius: 4px;
            margin-top: 8px;
            white-space: pre-wrap;
            word-break: break-all;
        }
    `);

    // ===================== 数据提取 =====================
    function extractEventData() {
        const content = document.querySelector('.mw-parser-output') || document.querySelector('main');
        if (!content) return null;

        const title = document.querySelector('h1#firstHeading, h1.page-header__title, .mw-page-title-main');
        const eventName = title ? title.textContent.trim() : 'unknown';
        const wikiId = decodeURIComponent(window.location.pathname.split('/').pop());

        function extractSection(sectionId) {
            // 方法1：通过 span id
            let header = content.querySelector(`span#${sectionId}`);
            if (header) header = header.closest('h2');

            // 方法2：通过标题文本
            if (!header) {
                for (const h of content.querySelectorAll('h2')) {
                    if (h.textContent.includes(sectionId)) { header = h; break; }
                }
            }
            if (!header) return '';

            const lines = [];
            let sibling = header.nextElementSibling;
            while (sibling && sibling.tagName !== 'H2') {
                if (sibling.tagName === 'UL' || sibling.tagName === 'OL') {
                    sibling.querySelectorAll('li').forEach(li => {
                        lines.push('• ' + li.textContent.trim());
                    });
                } else {
                    const text = sibling.textContent.trim();
                    if (text) lines.push(text);
                }
                sibling = sibling.nextElementSibling;
            }
            return lines.join('\n');
        }

        return {
            wiki_id: wikiId,
            name: eventName,
            url: window.location.href,
            scraped_at: new Date().toISOString(),
            raw_options: extractSection('Options'),
            raw_dialogue: extractSection('Dialogue'),
            raw_notes: extractSection('Notes'),
            raw_trivia: extractSection('Trivia')
        };
    }

    // ===================== 存储管理 =====================
    function getStoredEvents() {
        try {
            return JSON.parse(GM_getValue('sts_events', '{}'));
        } catch { return {}; }
    }

    function storeEvent(data) {
        const events = getStoredEvents();
        events[data.wiki_id] = data;
        GM_setValue('sts_events', JSON.stringify(events));
        return Object.keys(events).length;
    }

    function exportAllEvents() {
        const events = getStoredEvents();
        const blob = new Blob([JSON.stringify(events, null, 2)], {type: 'application/json'});
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `sts_events_${new Date().toISOString().slice(0,10)}.json`;
        a.click();
        URL.revokeObjectURL(url);
        return Object.keys(events).length;
    }

    // ===================== UI =====================
    function createPanel() {
        const panel = document.createElement('div');
        panel.id = 'sts-extractor-panel';

        const stored = getStoredEvents();
        const storedCount = Object.keys(stored).length;
        const currentPage = decodeURIComponent(window.location.pathname.split('/').pop());
        const isEventPage = EVENT_PAGES.some(p => decodeURIComponent(p) === currentPage || p === currentPage);
        const isAlreadyStored = stored[currentPage];

        panel.innerHTML = `
            <button class="toggle-btn" title="最小化">−</button>
            <h3>🗡️ STS Event Extractor</h3>
            
            <button class="btn btn-primary" id="sts-extract-btn">
                📥 提取当前页面
            </button>
            
            <button class="btn btn-secondary" id="sts-copy-btn">
                📋 复制到剪贴板
            </button>
            
            <button class="btn btn-secondary" id="sts-export-btn">
                💾 导出全部 (${storedCount}/${EVENT_PAGES.length})
            </button>
            
            <button class="btn btn-secondary" id="sts-clear-btn">
                🗑️ 清空存储
            </button>
            
            <div class="progress-bar">
                <div class="fill" style="width: ${(storedCount / EVENT_PAGES.length * 100).toFixed(1)}%"></div>
            </div>
            
            <div class="status" id="sts-status">
                ${isEventPage 
                    ? (isAlreadyStored ? '✅ 此页面已提取' : '⏳ 等待提取...') 
                    : '⚠️ 非事件页面'}
            </div>
            
            <div class="info">
                当前: ${currentPage.substring(0, 30)}${currentPage.length > 30 ? '...' : ''}
            </div>
        `;

        document.body.appendChild(panel);

        // 事件绑定
        panel.querySelector('.toggle-btn').onclick = () => {
            panel.classList.toggle('minimized');
            panel.querySelector('.toggle-btn').textContent = panel.classList.contains('minimized') ? '+' : '−';
        };

        panel.querySelector('#sts-extract-btn').onclick = () => {
            const data = extractEventData();
            if (data && (data.raw_options || data.raw_dialogue)) {
                const count = storeEvent(data);
                updateStatus(`✅ 已保存! (${count}/${EVENT_PAGES.length})`, 'success');
                panel.querySelector('#sts-export-btn').textContent = `💾 导出全部 (${count}/${EVENT_PAGES.length})`;
                panel.querySelector('.progress-bar .fill').style.width = `${(count / EVENT_PAGES.length * 100)}%`;
                
                // 显示预览
                showPreview(data);
            } else {
                updateStatus('❌ 提取失败或无数据', 'error');
            }
        };

        panel.querySelector('#sts-copy-btn').onclick = () => {
            const data = extractEventData();
            if (data) {
                GM_setClipboard(JSON.stringify(data, null, 2));
                updateStatus('📋 已复制到剪贴板', 'success');
            }
        };

        panel.querySelector('#sts-export-btn').onclick = () => {
            const count = exportAllEvents();
            updateStatus(`💾 已导出 ${count} 个事件`, 'success');
        };

        panel.querySelector('#sts-clear-btn').onclick = () => {
            if (confirm('确定要清空所有已存储的事件数据吗？')) {
                GM_setValue('sts_events', '{}');
                updateStatus('🗑️ 已清空', 'success');
                panel.querySelector('#sts-export-btn').textContent = `💾 导出全部 (0/${EVENT_PAGES.length})`;
                panel.querySelector('.progress-bar .fill').style.width = '0%';
            }
        };

        function updateStatus(msg, type = '') {
            const status = panel.querySelector('#sts-status');
            status.textContent = msg;
            status.className = 'status ' + type;
        }

        function showPreview(data) {
            let preview = panel.querySelector('.extracted-preview');
            if (!preview) {
                preview = document.createElement('div');
                preview.className = 'extracted-preview';
                panel.appendChild(preview);
            }
            preview.textContent = `Options:\n${(data.raw_options || '(无)').substring(0, 200)}...\n\nDialogue:\n${(data.raw_dialogue || '(无)').substring(0, 200)}...`;
        }

        // 如果是事件页面，自动提取
        if (isEventPage && !isAlreadyStored) {
            setTimeout(() => {
                panel.querySelector('#sts-extract-btn').click();
            }, 500);
        }
    }

    // ===================== 初始化 =====================
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', createPanel);
    } else {
        createPanel();
    }
})();
