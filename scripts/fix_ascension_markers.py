"""
修复事件JSON中丢失的进阶(Ascension)标记

Wiki页面中使用图标来标记进阶等级，爬取时图标丢失，导致格式如：
- "12.5% (18%) 15" 实际应该是 "12.5% (A15+: 18%)"
- "50 (25) 15 Gold" 实际应该是 "50 (A15+: 25) Gold"
- "5 (3) 15" 实际应该是 "5 (A15+: 3)"

此脚本自动修复这些模式。
"""

import json
import re
from pathlib import Path

# 定义需要修复的文件
DATA_DIR = Path(__file__).parent.parent / "data" / "events_preprocessed"

def fix_ascension_markers(text: str) -> str:
    """修复文本中的进阶标记"""
    
    # Pattern 1: 数字% (数字%) 15 -> 数字% (A15+: 数字%)
    # 例如: 12.5% (18%) 15 -> 12.5% (A15+: 18%)
    text = re.sub(
        r'(\d+(?:\.\d+)?%)\s*\((\d+(?:\.\d+)?%)\)\s*15\b',
        r'\1 (A15+: \2)',
        text
    )
    
    # Pattern 2: 数字 (数字) 15 后面跟单位词 -> 数字 (A15+: 数字) 单位词
    # 例如: 50 (25) 15 Gold -> 50 (A15+: 25) Gold
    # 例如: 5 (3) 15  Apparitions -> 5 (A15+: 3) Apparitions
    text = re.sub(
        r'(\d+)\s*\((\d+)\)\s*15\s+(\S)',
        r'\1 (A15+: \2) \3',
        text
    )
    
    # Pattern 3: 数字 (数字) 15 在句尾或标点前 -> 数字 (A15+: 数字)
    # 例如: 3 (5) 15. -> 3 (A15+: 5).
    text = re.sub(
        r'(\d+)\s*\((\d+)\)\s*15([.,;:\s]|$)',
        r'\1 (A15+: \2)\3',
        text
    )
    
    # Pattern 4: 范围模式 数字 and 数字 (数字 and 数字) 15
    # 例如: between 20 and 50 (35 and 75) 15 -> between 20 and 50 (A15+: 35 and 75)
    text = re.sub(
        r'(\d+\s+and\s+\d+)\s*\((\d+\s+and\s+\d+)\)\s*15\b',
        r'\1 (A15+: \2)',
        text
    )
    
    # Pattern 5: 50% (100%) 15 特殊情况
    text = re.sub(
        r'(\d+%)\s*\((\d+%)\)\s*15([:\s])',
        r'\1 (A15+: \2)\3',
        text
    )
    
    # Pattern 6: 表格格式 "数字 (数字 15)" -> "数字 (A15+: 数字)"
    # 例如: "3 (5 15)" -> "3 (A15+: 5)"
    text = re.sub(
        r'"(\d+)\s*\((\d+)\s*15\)"',
        r'"\1 (A15+: \2)"',
        text
    )
    
    # Pattern 7: 25% / 35% 15 形式 (表格中的概率)
    # 例如: (25% / 35% 15) -> (25% / A15+: 35%)
    text = re.sub(
        r'\((\d+%)\s*/\s*(\d+%)\s*15\)',
        r'(\1 / A15+: \2)',
        text
    )
    
    # Pattern 8: A20 20 重复标记 -> A20
    # 例如: on A20 20 -> on A20
    text = re.sub(
        r'\bA20\s+20\b',
        r'A20',
        text
    )
    
    # Pattern 9: ) 20 格式 (A20图标丢失)
    # 例如: (or both bosses) 20 -> (or both bosses on A20)
    text = re.sub(
        r'\)\s+20\s+(will)',
        r' on A20) \1',
        text
    )
    
    return text


def fix_json_file(filepath: Path) -> tuple[int, list[str]]:
    """修复单个JSON文件，返回修改数量和修改详情"""
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    original_content = content
    
    # 应用修复
    fixed_content = fix_ascension_markers(content)
    
    # 统计修改
    changes = []
    if original_content != fixed_content:
        # 找出所有差异
        orig_lines = original_content.split('\n')
        fixed_lines = fixed_content.split('\n')
        
        for i, (orig, fixed) in enumerate(zip(orig_lines, fixed_lines), 1):
            if orig != fixed:
                changes.append(f"  Line {i}: '{orig.strip()}' -> '{fixed.strip()}'")
        
        # 保存修复后的文件
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(fixed_content)
    
    return len(changes), changes


def main():
    print("=" * 60)
    print("修复事件JSON中的进阶(Ascension)标记")
    print("=" * 60)
    
    total_changes = 0
    
    for json_file in sorted(DATA_DIR.glob("*.json")):
        print(f"\n处理文件: {json_file.name}")
        
        count, changes = fix_json_file(json_file)
        total_changes += count
        
        if count > 0:
            print(f"  修改了 {count} 处:")
            for change in changes[:10]:  # 只显示前10个
                print(change)
            if len(changes) > 10:
                print(f"  ... 还有 {len(changes) - 10} 处修改")
        else:
            print("  无需修改")
    
    print("\n" + "=" * 60)
    print(f"总计修复: {total_changes} 处")
    print("=" * 60)


if __name__ == "__main__":
    main()
