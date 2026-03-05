# Wiki 事件数据半自动抓取工具

由于 wiki.gg 和 fandom 有 Cloudflare 保护，使用半自动方式抓取。

## 🎯 推荐方案：油猴脚本（最简单）

### 安装步骤

1. 安装 [Tampermonkey](https://www.tampermonkey.net/) 浏览器扩展
2. 点击 Tampermonkey 图标 → 创建新脚本
3. 将 `sts_event_extractor.user.js` 的内容粘贴进去
4. Ctrl+S 保存

### 使用方法

1. 打开任意 wiki.gg 事件页面（如 https://slaythespire.wiki.gg/wiki/Big_Fish）
2. 右上角会出现红色面板
3. 脚本会**自动提取**事件页面数据并存储
4. 浏览完所有事件页面后，点击 **"💾 导出全部"** 下载 JSON

### 功能特点

- ✅ **自动检测**事件页面并提取
- ✅ **浏览器内存储**，不丢失进度
- ✅ **进度条显示**已抓取数量
- ✅ **一键导出**所有数据为 JSON
- ✅ 支持 wiki.gg 和 fandom 两个站点

---

## 备选方案：书签脚本（手动）

### 步骤 1：安装浏览器书签

1. 打开 `bookmarklet.js` 文件
2. 复制里面的代码
3. 在浏览器中创建新书签，将代码粘贴到 URL 栏

### 步骤 2：抓取事件页面

1. 打开 `event_urls.txt` 中的每个 URL
2. 页面加载完成后，点击书签
3. 会自动下载一个 `{event_name}.json` 文件
4. 将下载的文件移动到 `downloaded/` 文件夹

### 步骤 3：合并数据

```bash
python merge_events.py
```

---

## 文件说明

| 文件 | 说明 |
|------|------|
| `sts_event_extractor.user.js` | 🌟 **油猴脚本**（推荐） |
| `bookmarklet.js` | 浏览器书签脚本（单行版本） |
| `extractor.js` | 完整版提取脚本（可读版本） |
| `event_urls.txt` | 所有事件页面 URL 列表（51个） |
| `merge_events.py` | 合并所有下载的 JSON |
| `downloaded/` | 存放下载的 JSON 文件 |
