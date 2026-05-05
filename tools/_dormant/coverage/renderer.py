"""Generate static HTML coverage report."""
import html
from pathlib import Path
from .models import CategorySummary, CoverageEntry, HookStatus, EntityCategory


def _status_class(status: HookStatus) -> str:
    return {
        HookStatus.IMPLEMENTED: "hook-done",
        HookStatus.MISSING: "hook-missing",
        HookStatus.SKIPPED: "hook-skip",
        HookStatus.INLINE: "hook-done",
    }[status]


def _status_icon(status: HookStatus) -> str:
    return {
        HookStatus.IMPLEMENTED: "✅",
        HookStatus.MISSING: "❌",
        HookStatus.SKIPPED: "⏭",
        HookStatus.INLINE: "🔧",
    }[status]


def _pct_bar(pct: float) -> str:
    color = "#22c55e" if pct >= 100 else "#eab308" if pct > 0 else "#ef4444"
    return f'''<div class="pct-bar"><div class="pct-fill" style="width:{pct:.0f}%;background:{color}"></div><span class="pct-text">{pct:.0f}%</span></div>'''


def _entry_row(entry: CoverageEntry, java_root: str) -> str:
    """Generate HTML for one entity row (collapsible)."""
    e = entry
    icon = e.status_icon
    name = html.escape(e.java.class_name)
    java_id = html.escape(e.java.java_id)
    java_file = e.java.java_file.replace("\\", "/")
    java_path = f"{java_root}/{java_file}" if java_file else ""
    rust_path = e.rust.file_path.replace("\\", "/") if e.rust and e.rust.file_path else ""
    scattered = "⚠️ 散布逻辑" if e.java.has_scattered_logic else ""

    hooks_html = ""
    for h in e.hook_details:
        sc = _status_class(h.status)
        si = _status_icon(h.status)
        rust_fn = html.escape(h.rust_function or "—")
        hooks_html += f'<tr class="{sc}"><td>{si} {html.escape(h.name)}</td><td><code>{rust_fn}</code></td></tr>\n'

    impl = e.implemented_hooks
    total = e.total_hooks
    pct = e.coverage_pct

    return f'''
    <details class="entity-card" {"open" if pct < 100 and total > 0 else ""}>
      <summary>
        <span class="entity-icon">{icon}</span>
        <span class="entity-name">{name}</span>
        <span class="entity-id">({java_id})</span>
        <span class="entity-pct">{impl}/{total}</span>
        {_pct_bar(pct)}
        <span class="scattered">{scattered}</span>
      </summary>
      <div class="entity-detail">
        <div class="file-links">
          {"<a href='file:///" + java_path + "'>📄 Java</a>" if java_path else ""}
          {"<a href='file:///" + rust_path + "'>🦀 Rust</a>" if rust_path else "<span class='no-rust'>🦀 无 Rust 文件</span>"}
        </div>
        <table class="hook-table">
          <tr><th>Java Hook</th><th>Rust 函数</th></tr>
          {hooks_html}
        </table>
      </div>
    </details>'''


def render_html(summaries: dict[EntityCategory, CategorySummary],
                output_path: Path, java_root: str = "d:/rust/cardcrawl") -> None:
    """Generate the full HTML report."""

    category_tabs = ""
    category_content = ""

    for cat, summary in summaries.items():
        cat_name = cat.value.title()
        cat_id = cat.value

        # Build entries
        entries_html = ""
        for entry in sorted(summary.entries, key=lambda e: (e.coverage_pct >= 100, e.java.class_name)):
            entries_html += _entry_row(entry, java_root)

        category_tabs += f'<button class="tab-btn" onclick="showTab(\'{cat_id}\')" id="tab-{cat_id}">{cat_name}s ({summary.fully_covered}/{summary.total_java})</button>\n'

        category_content += f'''
        <div class="tab-content" id="content-{cat_id}">
          <div class="summary-bar">
            <div class="stat"><span class="stat-num">{summary.total_java}</span><span class="stat-label">Java 总数</span></div>
            <div class="stat"><span class="stat-num">{summary.has_rust_file}</span><span class="stat-label">有 .rs 文件</span></div>
            <div class="stat good"><span class="stat-num">{summary.fully_covered}</span><span class="stat-label">完全覆盖</span></div>
            <div class="stat warn"><span class="stat-num">{summary.partially_covered}</span><span class="stat-label">部分覆盖</span></div>
            <div class="stat bad"><span class="stat-num">{summary.not_covered}</span><span class="stat-label">未覆盖</span></div>
          </div>
          <div class="filter-bar">
            <input type="text" id="search-{cat_id}" placeholder="搜索..." oninput="filterEntries('{cat_id}')">
            <label><input type="checkbox" id="hide-done-{cat_id}" onchange="filterEntries('{cat_id}')"> 隐藏已完成</label>
          </div>
          <div class="entries" id="entries-{cat_id}">
            {entries_html}
          </div>
        </div>'''

    full_html = f'''<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="UTF-8">
<title>STS Coverage Dashboard</title>
<style>
:root {{ --bg: #0f172a; --card: #1e293b; --border: #334155; --text: #e2e8f0; --text2: #94a3b8; --green: #22c55e; --yellow: #eab308; --red: #ef4444; }}
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{ background: var(--bg); color: var(--text); font-family: 'Segoe UI', system-ui, sans-serif; padding: 20px; }}
h1 {{ text-align: center; font-size: 1.8em; margin-bottom: 20px; background: linear-gradient(135deg, #6366f1, #8b5cf6); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }}
.tabs {{ display: flex; gap: 8px; margin-bottom: 16px; justify-content: center; }}
.tab-btn {{ background: var(--card); border: 1px solid var(--border); color: var(--text); padding: 10px 24px; border-radius: 8px; cursor: pointer; font-size: 1em; transition: all 0.2s; }}
.tab-btn:hover {{ border-color: #6366f1; }}
.tab-btn.active {{ background: #6366f1; border-color: #6366f1; font-weight: 600; }}
.tab-content {{ display: none; }}
.tab-content.active {{ display: block; }}
.summary-bar {{ display: flex; gap: 16px; justify-content: center; margin-bottom: 16px; flex-wrap: wrap; }}
.stat {{ background: var(--card); border: 1px solid var(--border); border-radius: 8px; padding: 12px 20px; text-align: center; min-width: 100px; }}
.stat-num {{ display: block; font-size: 1.6em; font-weight: 700; }}
.stat-label {{ font-size: 0.8em; color: var(--text2); }}
.stat.good .stat-num {{ color: var(--green); }}
.stat.warn .stat-num {{ color: var(--yellow); }}
.stat.bad .stat-num {{ color: var(--red); }}
.filter-bar {{ display: flex; gap: 12px; margin-bottom: 16px; align-items: center; }}
.filter-bar input[type=text] {{ background: var(--card); border: 1px solid var(--border); color: var(--text); padding: 8px 16px; border-radius: 6px; flex: 1; max-width: 400px; font-size: 0.95em; }}
.filter-bar label {{ color: var(--text2); font-size: 0.9em; cursor: pointer; white-space: nowrap; }}
.entries {{ display: flex; flex-direction: column; gap: 4px; }}
.entity-card {{ background: var(--card); border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }}
.entity-card summary {{ display: flex; align-items: center; gap: 8px; padding: 10px 16px; cursor: pointer; user-select: none; }}
.entity-card summary:hover {{ background: #253249; }}
.entity-icon {{ font-size: 1em; }}
.entity-name {{ font-weight: 600; min-width: 200px; }}
.entity-id {{ color: var(--text2); font-size: 0.85em; min-width: 150px; }}
.entity-pct {{ color: var(--text2); font-size: 0.85em; min-width: 40px; text-align: right; }}
.pct-bar {{ width: 100px; height: 6px; background: #334155; border-radius: 3px; position: relative; margin-left: 8px; }}
.pct-fill {{ height: 100%; border-radius: 3px; transition: width 0.3s; }}
.pct-text {{ position: absolute; right: -35px; top: -7px; font-size: 0.75em; color: var(--text2); }}
.scattered {{ color: var(--yellow); font-size: 0.8em; margin-left: auto; }}
.entity-detail {{ padding: 0 16px 12px; }}
.file-links {{ display: flex; gap: 12px; margin-bottom: 8px; }}
.file-links a {{ color: #818cf8; text-decoration: none; font-size: 0.85em; }}
.file-links a:hover {{ text-decoration: underline; }}
.no-rust {{ color: var(--red); font-size: 0.85em; }}
.hook-table {{ width: 100%; border-collapse: collapse; font-size: 0.85em; }}
.hook-table th {{ text-align: left; padding: 4px 8px; border-bottom: 1px solid var(--border); color: var(--text2); }}
.hook-table td {{ padding: 4px 8px; border-bottom: 1px solid #1e293b; }}
.hook-done {{ color: var(--green); }}
.hook-missing {{ color: var(--red); }}
.hook-skip {{ color: var(--text2); opacity: 0.6; }}
</style>
</head>
<body>
<h1>🗡️ STS Java ↔ Rust Coverage Dashboard</h1>
<div class="tabs">{category_tabs}</div>
{category_content}
<script>
function showTab(id) {{
  document.querySelectorAll('.tab-content').forEach(el => el.classList.remove('active'));
  document.querySelectorAll('.tab-btn').forEach(el => el.classList.remove('active'));
  document.getElementById('content-' + id).classList.add('active');
  document.getElementById('tab-' + id).classList.add('active');
}}
function filterEntries(catId) {{
  const q = document.getElementById('search-' + catId).value.toLowerCase();
  const hideDone = document.getElementById('hide-done-' + catId).checked;
  document.querySelectorAll('#entries-' + catId + ' .entity-card').forEach(card => {{
    const name = card.querySelector('.entity-name').textContent.toLowerCase();
    const id = card.querySelector('.entity-id').textContent.toLowerCase();
    const isDone = card.querySelector('.entity-icon').textContent.includes('✅');
    let show = (name.includes(q) || id.includes(q));
    if (hideDone && isDone) show = false;
    card.style.display = show ? '' : 'none';
  }});
}}
// Show first tab by default
document.querySelector('.tab-btn')?.click();
</script>
</body>
</html>'''

    output_path.write_text(full_html, encoding="utf-8")
