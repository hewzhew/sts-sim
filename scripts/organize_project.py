"""Organize project files: move temp/old/one-off files to _archive."""
import shutil
import os

PROJECT = '.'

# Create archive directories
archives = {
    '_archive/temp_outputs':    'Temporary output/debug files',
    '_archive/old_data':        'Legacy/duplicate data files',
    '_archive/one_off_scripts': 'One-off analysis/scraping scripts',
    '_archive/old_tools':       'Superseded tool versions',
    '_archive/rl_training':     'RL training artifacts (models, logs, configs)',
}

for d, desc in archives.items():
    os.makedirs(os.path.join(PROJECT, d), exist_ok=True)

moves = []

def mv(src, dst_dir, reason):
    """Queue a move operation."""
    moves.append((src, dst_dir, reason))

# ============================================================================
# ROOT - temp outputs (无需保留)
# ============================================================================
mv('validate_output.txt',      '_archive/temp_outputs', 'validate_cards的临时输出')
mv('card_defs.txt',            '_archive/temp_outputs', 'show_cards的临时输出')
mv('test_output.txt',          '_archive/temp_outputs', '测试输出日志 113KB')
mv('test_results.txt',         '_archive/temp_outputs', '测试结果日志 113KB')
mv('test_err.txt',             '_archive/temp_outputs', '测试错误日志')
mv('batch2a_cards.txt',        '_archive/temp_outputs', 'batch 2a 卡牌临时列表')
mv('incomplete_cards.json',    '_archive/temp_outputs', '未完成卡牌的临时JSON')
mv('card_analysis_report.html','_archive/temp_outputs', '卡牌分析HTML报告 234KB')
mv('cards.json',               '_archive/temp_outputs', '根目录残留的cards.json副本')

# ============================================================================
# ROOT - RL training files (不常用)
# ============================================================================
mv('ppo_sts_run1.zip',         '_archive/rl_training', 'PPO模型检查点')
mv('ppo_sts_v1.zip',           '_archive/rl_training', 'PPO模型检查点')
mv('train_ppo.py',             '_archive/rl_training', 'PPO训练脚本')
mv('train_maskable_ppo.py',    '_archive/rl_training', 'Maskable PPO训练脚本')
mv('muzero_arch.py',           '_archive/rl_training', 'MuZero架构')
mv('enjoy.py',                 '_archive/rl_training', 'RL模型评测')
mv('demo.py',                  '_archive/rl_training', 'RL演示脚本')
mv('interactive_demo.py',      '_archive/rl_training', 'RL交互演示')
mv('gym_env.py',               '_archive/rl_training', 'Gym环境封装')
mv('check_progress.py',        '_archive/rl_training', '训练进度检查')

# ROOT - test files (RL相关测试)
mv('test_action_mapping.py',   '_archive/rl_training', 'RL动作映射测试')
mv('test_boss_map.py',         '_archive/rl_training', 'Boss地图测试')
mv('test_encoding.py',         '_archive/rl_training', '编码测试')
mv('test_env.py',              '_archive/rl_training', '环境测试')
mv('test_gym.py',              '_archive/rl_training', 'Gym测试')
mv('test_reward.py',           '_archive/rl_training', '奖励函数测试')
mv('test_rl_loop.py',          '_archive/rl_training', 'RL循环测试')
mv('test_serde.rs',            '_archive/rl_training', 'Serde测试(根目录残留)')

# ROOT - one-off analysis scripts
mv('analyze_cards.py',         '_archive/one_off_scripts', '卡牌分析脚本 (已被validate_cards替代)')
mv('edit_cards.py',            '_archive/one_off_scripts', '卡牌编辑脚本 (ETL流程)')

# ============================================================================
# DATA - old/duplicate data files
# ============================================================================
mv('data/cards.json',          '_archive/old_data', '旧单文件cards.json (已拆分到data/cards/)')
mv('data/cards_patched.json',  '_archive/old_data', '旧patched版本 (已合并到data/cards/)')
mv('data/cards.lua',           '_archive/old_data', 'Lua格式卡牌数据 (原始导出)')
mv('data/cards.txt',           '_archive/old_data', '文本格式卡牌数据 (原始导出)')
mv('data/keywords.lua',        '_archive/old_data', 'Lua格式关键词')
mv('data/relics.txt',          '_archive/old_data', '文本格式遗物数据')
mv('data/potions.txt',         '_archive/old_data', '文本格式药水数据')
mv('data/monsters_lua.txt',    '_archive/old_data', 'Lua格式怪物数据')

# DATA - ETL/processing artifacts
mv('data/ai_descriptions_extracted.json', '_archive/old_data', 'AI描述提取结果')
mv('data/extract_ai_desc.py',  '_archive/old_data', 'AI描述提取脚本')
mv('data/extract_json_schema.py', '_archive/old_data', 'Schema提取脚本')
mv('data/etl_behavior_model.py', '_archive/old_data', 'ETL行为模型')
mv('data/update_missing_ai.py', '_archive/old_data', 'AI缺失数据更新')
mv('data/merge_monsters.py',   '_archive/old_data', '怪物合并脚本')

# DATA - intermediate monster files
mv('data/monsters_boss_sheet.tsv', '_archive/old_data', 'Boss怪物表格')
mv('data/monsters_sheet.tsv',  '_archive/old_data', '一般怪物表格')
mv('data/monsters_json.json',  '_archive/old_data', '怪物JSON中间产物 280KB')

# DATA - old event scrape data
mv('data/sts_events_2026-01-23.json', '_archive/old_data', '旧事件抓取数据')

# ============================================================================
# SCRIPTS - one-off scripts (已完成用途)
# ============================================================================
mv('scripts/show_cards.py',    '_archive/one_off_scripts', '临时卡牌显示')
mv('scripts/list_red_common.py', '_archive/one_off_scripts', '临时红色common列表')
mv('scripts/run_validate.py',  '_archive/one_off_scripts', '临时验证运行器')

# ============================================================================
# TOOLS - superseded versions
# ============================================================================
mv('tools/event_editor.html',  '_archive/old_tools', '事件编辑器v1')
mv('tools/parse_cards.py',     '_archive/old_tools', '卡牌解析器v1')
mv('tools/parse_cards_v2.py',  '_archive/old_tools', '卡牌解析器v2 (已被ETL流程替代)')
mv('tools/parse_relics.py',    '_archive/old_tools', '遗物解析器')
mv('tools/patch_cards.py',     '_archive/old_tools', '卡牌补丁脚本 92KB (已完成)')

# ============================================================================
# DIRECTORIES to move
# ============================================================================
mv('__pycache__',              '_archive/temp_outputs', 'Python缓存')
mv('best_model',               '_archive/rl_training', 'RL最佳模型')
mv('checkpoints',              '_archive/rl_training', 'RL检查点')
mv('eval_logs',                '_archive/rl_training', 'RL评估日志')
mv('logs',                     '_archive/rl_training', 'RL训练日志')
mv('tensorboard_logs',         '_archive/rl_training', 'TensorBoard日志')
mv('slay_the_spire',           '_archive/rl_training', 'STS Python模块')

# ============================================================================
# Execute moves
# ============================================================================
moved_count = 0
failed = []

for src, dst_dir, reason in moves:
    src_path = os.path.join(PROJECT, src)
    if not os.path.exists(src_path):
        continue
    
    dst_path = os.path.join(PROJECT, dst_dir, os.path.basename(src))
    try:
        shutil.move(src_path, dst_path)
        moved_count += 1
        print(f"  ✅ {src:45s} → {dst_dir}/")
    except Exception as e:
        failed.append((src, str(e)))
        print(f"  ❌ {src:45s} ERROR: {e}")

print(f"\nMoved: {moved_count} items")
if failed:
    print(f"Failed: {len(failed)} items")
    for f, e in failed:
        print(f"  {f}: {e}")
