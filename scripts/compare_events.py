#!/usr/bin/env python3
"""
事件数据对比脚本

将预处理后的 Wiki 数据与现有 events_logic.json 对比，
生成差异报告供 AI 或人工审核。
"""

import json
from pathlib import Path
from difflib import unified_diff

DATA_DIR = Path(__file__).parent.parent / "data"
PREPROCESSED_DIR = DATA_DIR / "events_preprocessed"
EXISTING_LOGIC = DATA_DIR / "events_logic.json"
EXISTING_EVENTS = DATA_DIR / "events.json"
DIFF_OUTPUT = DATA_DIR / "events_diff_report"


def normalize_name(name: str) -> str:
    """规范化事件名称用于匹配"""
    return name.lower().replace(" ", "_").replace("'", "").replace("!", "").replace("-", "_")


def load_existing_events():
    """加载现有的事件数据"""
    logic = {}
    if EXISTING_LOGIC.exists():
        with open(EXISTING_LOGIC, 'r', encoding='utf-8') as f:
            logic = json.load(f)
    
    events = {}
    if EXISTING_EVENTS.exists():
        with open(EXISTING_EVENTS, 'r', encoding='utf-8') as f:
            events = json.load(f)
    
    return logic, events


def compare_options(wiki_options: list, existing_options: list) -> dict:
    """比较选项列表"""
    diff = {
        "wiki_count": len(wiki_options),
        "existing_count": len(existing_options),
        "matches": [],
        "wiki_only": [],
        "existing_only": [],
        "differences": []
    }
    
    # 简单按标签匹配
    wiki_labels = {opt.get('label', '').lower(): opt for opt in wiki_options}
    existing_labels = {opt.get('label', '').lower(): opt for opt in existing_options}
    
    for label, wiki_opt in wiki_labels.items():
        if label in existing_labels:
            diff["matches"].append(label)
            # 检查具体差异
            exist_opt = existing_labels[label]
            if wiki_opt.get('description') != exist_opt.get('description', ''):
                diff["differences"].append({
                    "label": label,
                    "field": "description",
                    "wiki": wiki_opt.get('description'),
                    "existing": exist_opt.get('description', '')
                })
        else:
            diff["wiki_only"].append(wiki_opt)
    
    for label, exist_opt in existing_labels.items():
        if label not in wiki_labels:
            diff["existing_only"].append(exist_opt)
    
    return diff


def generate_diff_report(category: str):
    """生成单个分类的差异报告"""
    preprocessed_file = PREPROCESSED_DIR / f"{category}.json"
    if not preprocessed_file.exists():
        print(f"  [跳过] {category} - 预处理文件不存在")
        return None
    
    with open(preprocessed_file, 'r', encoding='utf-8') as f:
        wiki_events = json.load(f)
    
    logic, events = load_existing_events()
    
    report = {
        "category": category,
        "total_wiki_events": len(wiki_events),
        "events": []
    }
    
    for wiki_event in wiki_events:
        wiki_id = wiki_event['wiki_id']
        name = wiki_event['name']
        
        # 在现有数据中查找匹配
        existing_event = None
        for key, val in logic.items():
            if normalize_name(key) == normalize_name(name) or \
               normalize_name(key) == normalize_name(wiki_id):
                existing_event = val
                break
        
        event_report = {
            "wiki_id": wiki_id,
            "name": name,
            "found_in_existing": existing_event is not None,
            "wiki_options_count": len(wiki_event['options']),
            "existing_options_count": len(existing_event.get('options', [])) if existing_event else 0,
            "option_labels_wiki": [opt['label'] for opt in wiki_event['options']],
            "option_labels_existing": [opt.get('label', '') for opt in existing_event.get('options', [])] if existing_event else [],
            "needs_review": False,
            "issues": []
        }
        
        # 检查问题
        if not existing_event:
            event_report["needs_review"] = True
            event_report["issues"].append("事件在 events_logic.json 中不存在")
        elif event_report["wiki_options_count"] != event_report["existing_options_count"]:
            event_report["needs_review"] = True
            event_report["issues"].append(f"选项数量不匹配: Wiki={event_report['wiki_options_count']}, 现有={event_report['existing_options_count']}")
        
        # 比较选项标签
        if existing_event:
            wiki_labels = set(opt['label'].lower() for opt in wiki_event['options'])
            exist_labels = set(opt.get('label', '').lower() for opt in existing_event.get('options', []))
            
            missing_in_existing = wiki_labels - exist_labels
            extra_in_existing = exist_labels - wiki_labels
            
            if missing_in_existing:
                event_report["needs_review"] = True
                event_report["issues"].append(f"现有数据缺少选项: {missing_in_existing}")
            if extra_in_existing:
                event_report["needs_review"] = True
                event_report["issues"].append(f"现有数据多余选项: {extra_in_existing}")
        
        report["events"].append(event_report)
    
    return report


def main():
    """主函数"""
    print("=" * 60)
    print("事件数据对比分析")
    print("=" * 60)
    
    DIFF_OUTPUT.mkdir(parents=True, exist_ok=True)
    
    categories = ["shrines", "act1", "act2", "act3"]
    all_reports = {}
    total_needs_review = 0
    
    for category in categories:
        print(f"\n分析 {category}...")
        report = generate_diff_report(category)
        
        if report:
            all_reports[category] = report
            
            # 保存分类报告
            output_file = DIFF_OUTPUT / f"{category}_diff.json"
            with open(output_file, 'w', encoding='utf-8') as f:
                json.dump(report, f, indent=2, ensure_ascii=False)
            
            # 统计需要审核的事件
            needs_review = [e for e in report["events"] if e["needs_review"]]
            total_needs_review += len(needs_review)
            
            print(f"  事件数: {len(report['events'])}")
            print(f"  需要审核: {len(needs_review)}")
            
            if needs_review:
                print("  需要审核的事件:")
                for e in needs_review[:3]:  # 只显示前3个
                    print(f"    - {e['name']}: {'; '.join(e['issues'])}")
                if len(needs_review) > 3:
                    print(f"    ... 还有 {len(needs_review) - 3} 个")
    
    # 生成汇总报告
    summary = {
        "total_events": sum(len(r["events"]) for r in all_reports.values()),
        "total_needs_review": total_needs_review,
        "by_category": {
            cat: {
                "total": len(report["events"]),
                "needs_review": len([e for e in report["events"] if e["needs_review"]])
            }
            for cat, report in all_reports.items()
        }
    }
    
    summary_file = DIFF_OUTPUT / "summary.json"
    with open(summary_file, 'w', encoding='utf-8') as f:
        json.dump(summary, f, indent=2, ensure_ascii=False)
    
    print("\n" + "=" * 60)
    print("对比完成！")
    print("=" * 60)
    print(f"\n总事件数: {summary['total_events']}")
    print(f"需要审核: {summary['total_needs_review']}")
    print(f"\n报告位置: {DIFF_OUTPUT}")
    print("\n下一步: 查看各分类的 *_diff.json 文件，逐个审核问题事件")


if __name__ == "__main__":
    main()
