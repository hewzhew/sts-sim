/**
 * Slay the Spire Wiki Event Extractor
 * 
 * 在 wiki.gg 事件页面的浏览器控制台中运行此脚本
 * 会自动提取 Options 和 Dialogue 部分并下载为 JSON
 */

(function() {
    'use strict';
    
    // 获取事件名称
    const title = document.querySelector('h1#firstHeading, h1.page-header__title, .mw-page-title-main');
    const eventName = title ? title.textContent.trim() : 'unknown_event';
    
    // 从 URL 提取 wiki_id
    const wikiId = window.location.pathname.split('/').pop();
    
    // 提取主内容区域
    const content = document.querySelector('.mw-parser-output') || document.querySelector('main');
    
    if (!content) {
        alert('无法找到页面内容！');
        return;
    }
    
    // 提取指定 section 的内容
    function extractSection(sectionId) {
        // 方法1：通过 span id 查找
        let header = content.querySelector(`span#${sectionId}`);
        if (header) {
            header = header.closest('h2');
        }
        
        // 方法2：通过标题文本查找
        if (!header) {
            const headers = content.querySelectorAll('h2');
            for (const h of headers) {
                if (h.textContent.includes(sectionId)) {
                    header = h;
                    break;
                }
            }
        }
        
        if (!header) return '';
        
        // 收集到下一个 h2 之前的所有内容
        const lines = [];
        let sibling = header.nextElementSibling;
        
        while (sibling && sibling.tagName !== 'H2') {
            // 清理文本
            let text = sibling.textContent.trim();
            if (text) {
                // 保留结构化信息
                if (sibling.tagName === 'UL' || sibling.tagName === 'OL') {
                    const items = sibling.querySelectorAll('li');
                    items.forEach(li => {
                        lines.push('• ' + li.textContent.trim());
                    });
                } else {
                    lines.push(text);
                }
            }
            sibling = sibling.nextElementSibling;
        }
        
        return lines.join('\n');
    }
    
    // 提取选项表格（如果存在）
    function extractOptionsTable() {
        const tables = content.querySelectorAll('table');
        for (const table of tables) {
            const headerRow = table.querySelector('tr');
            if (headerRow && headerRow.textContent.includes('Option')) {
                const rows = table.querySelectorAll('tr');
                const options = [];
                for (let i = 1; i < rows.length; i++) {
                    const cells = rows[i].querySelectorAll('td');
                    if (cells.length >= 2) {
                        options.push({
                            option: cells[0].textContent.trim(),
                            effect: cells[1].textContent.trim()
                        });
                    }
                }
                return options;
            }
        }
        return [];
    }
    
    // 构建结果对象
    const result = {
        wiki_id: wikiId,
        name: eventName,
        url: window.location.href,
        scraped_at: new Date().toISOString(),
        
        // 原始文本
        raw_options: extractSection('Options'),
        raw_dialogue: extractSection('Dialogue'),
        raw_notes: extractSection('Notes'),
        raw_trivia: extractSection('Trivia'),
        
        // 结构化数据（如果能提取）
        options_table: extractOptionsTable(),
        
        // 页面元信息
        categories: Array.from(document.querySelectorAll('.mw-normal-catlinks a'))
            .map(a => a.textContent.trim())
            .filter(c => c !== 'Categories'),
    };
    
    // 下载为 JSON 文件
    const blob = new Blob([JSON.stringify(result, null, 2)], {type: 'application/json'});
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${wikiId}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    
    // 显示提取结果预览
    console.log('=== Extracted Event Data ===');
    console.log('Name:', result.name);
    console.log('Options:', result.raw_options.substring(0, 200) + '...');
    console.log('Dialogue:', result.raw_dialogue.substring(0, 200) + '...');
    console.log('Downloaded:', `${wikiId}.json`);
    
    alert(`✅ 已提取并下载: ${wikiId}.json\n\n选项: ${result.raw_options ? '✓' : '✗'}\n对话: ${result.raw_dialogue ? '✓' : '✗'}`);
})();
