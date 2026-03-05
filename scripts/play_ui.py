"""
Interactive Play UI v2 — Browser-based Slay the Spire simulator.

Features:
  - Play cards by clicking, see card names + costs
  - Watch AI (MCTS) make decisions with card-name labels
  - Full game state: hand, enemies (HP bars + intents), pile sizes
  - History log with card names

Run: python scripts/play_ui.py
Open: http://localhost:5050
"""
import json, sys, argparse
import numpy as np

try:
    from flask import Flask, render_template_string, jsonify, request
except ImportError:
    print("Flask not installed. Run: pip install flask")
    sys.exit(1)

import sts_sim

app = Flask(__name__)

game = {'env': None, 'seed': 42, 'history': [], 'step_count': 0}

def init_game(seed=None):
    if seed is not None:
        game['seed'] = seed
    sts_sim.set_verbose(False)
    game['env'] = sts_sim.PyStsSim(seed=game['seed'])
    game['env'].reset(seed=game['seed'])
    game['history'] = []
    game['step_count'] = 0

def get_game_state():
    env = game['env']
    if env is None:
        return {'error': 'No game initialized'}

    screen = env.get_screen_type()
    mask = env.get_valid_actions_mask()
    valid = [i for i, v in enumerate(mask) if v]

    state = {
        'screen': screen,
        'hp': env.get_hp(),
        'max_hp': env.get_max_hp(),
        'gold': env.get_gold(),
        'act': env.get_act(),
        'floor': env.get_floor(),
        'step': game['step_count'],
        'valid_actions': valid,
        'seed': game['seed'],
    }

    try:
        obs = env.get_observation_dict()
        state['energy'] = obs.get('energy', 0)
        state['max_energy'] = obs.get('max_energy', 0)
        state['block'] = obs.get('block', 0)
        state['turn'] = obs.get('turn', 0)
        state['draw_pile_size'] = obs.get('draw_pile_size', 0)
        state['discard_pile_size'] = obs.get('discard_pile_size', 0)
        state['exhaust_pile_size'] = obs.get('exhaust_pile_size', 0)

        # Hand: list of card name strings
        hand_raw = obs.get('hand', [])
        hand_costs = obs.get('hand_costs', [])
        state['hand'] = [str(c) for c in hand_raw]
        state['hand_costs'] = [int(c) for c in hand_costs]

        # Enemies: list of tuples (hp, max_hp, block, intent_name, intent_dmg)
        enemies_raw = obs.get('enemies', [])
        enemies = []
        for e in enemies_raw:
            if isinstance(e, (list, tuple)) and len(e) >= 5:
                enemies.append({
                    'hp': e[0], 'max_hp': e[1], 'block': e[2],
                    'intent': str(e[3]), 'intent_dmg': e[4],
                })
            elif isinstance(e, dict):
                enemies.append(e)
        state['enemies'] = enemies

        # Pile contents for inspection
        state['draw_pile'] = [str(c) for c in obs.get('draw_pile', [])]
        state['discard_pile'] = [str(c) for c in obs.get('discard_pile', [])]
        state['master_deck'] = [str(c) for c in obs.get('master_deck', [])]
        state['rewards'] = [str(r) for r in obs.get('rewards', [])]
    except Exception as e:
        state['obs_error'] = str(e)
        state['hand'] = []
        state['hand_costs'] = []
        state['enemies'] = []

    # MCTS recommendation (combat only)
    if screen == 'COMBAT' and valid:
        try:
            mcts = env.mcts_evaluate(n_sims=20, max_turns=10)
            state['mcts_best'] = mcts['best_action']
            state['mcts_actions'] = mcts.get('actions', [])
        except:
            pass

    return state


def pick_noncombat(valid, screen):
    for a in [99, 39, 34, 35, 36]:
        if a in valid: return a
    for a in range(30, 34):
        if a in valid: return a
    for a in range(20, 30):
        if a in valid: return a
    for a in [60, 61, 62, 63, 64, 65]:
        if a in valid: return a
    for a in range(90, 100):
        if a in valid: return a
    return valid[0] if valid else 99


HTML_TEMPLATE = r"""
<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>STS Simulator — Interactive Play</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: 'Segoe UI', system-ui, sans-serif;
    background: #0d1117; color: #c9d1d9;
    min-height: 100vh;
  }
  .container { max-width: 1200px; margin: 0 auto; padding: 16px; }
  h1 { color: #58a6ff; margin-bottom: 8px; font-size: 1.3em; }
  .subtitle { color: #8b949e; font-size: 0.8em; margin-bottom: 12px; }

  .controls { display: flex; gap: 8px; margin-bottom: 12px; flex-wrap: wrap; align-items: center; }
  .ctrl-btn {
    background: #238636; border: none; color: white;
    padding: 7px 14px; border-radius: 6px; cursor: pointer;
    font-size: 0.82em; transition: background 0.15s;
  }
  .ctrl-btn:hover { background: #2ea043; }
  .ctrl-btn.danger { background: #da3633; }
  .ctrl-btn.danger:hover { background: #f85149; }
  .ctrl-btn.secondary { background: #30363d; }
  .ctrl-btn.secondary:hover { background: #484f58; }
  #seed-input {
    background: #21262d; border: 1px solid #30363d; color: #c9d1d9;
    padding: 6px 10px; border-radius: 6px; width: 80px; font-size: 0.82em;
  }

  .status-bar {
    display: flex; gap: 10px; flex-wrap: wrap;
    background: #161b22; border-radius: 8px;
    padding: 8px 14px; margin-bottom: 10px;
    border: 1px solid #30363d;
  }
  .stat { text-align: center; min-width: 60px; }
  .stat-label { font-size: 0.65em; color: #8b949e; text-transform: uppercase; letter-spacing: 0.5px; }
  .stat-value { font-size: 1.1em; font-weight: bold; }
  .hp-color { color: #f85149; }
  .energy-color { color: #f0c050; }
  .gold-color { color: #d2a038; }
  .block-color { color: #58a6ff; }
  .screen-tag {
    background: #238636; color: white; padding: 2px 10px;
    border-radius: 12px; font-weight: bold; font-size: 0.8em;
    align-self: center;
  }

  .panels { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-bottom: 10px; }
  .panel {
    background: #161b22; border: 1px solid #30363d;
    border-radius: 8px; padding: 10px;
  }
  .panel h3 { color: #58a6ff; font-size: 0.85em; margin-bottom: 6px; }

  /* Cards */
  .hand { display: flex; flex-wrap: wrap; gap: 5px; }
  .card {
    background: #21262d; border: 2px solid #30363d;
    border-radius: 6px; padding: 5px 8px; font-size: 0.78em;
    cursor: pointer; transition: all 0.15s; position: relative;
    min-width: 70px; text-align: center;
  }
  .card:hover { border-color: #58a6ff; transform: translateY(-3px); box-shadow: 0 4px 12px rgba(88,166,255,0.2); }
  .card .cost-badge {
    position: absolute; top: -6px; right: -6px;
    background: #f0c050; color: #000; width: 20px; height: 20px;
    border-radius: 50%; text-align: center; font-size: 0.7em;
    line-height: 20px; font-weight: bold;
  }
  .card.unplayable { opacity: 0.35; cursor: not-allowed; border-color: #21262d; }
  .card.unplayable:hover { transform: none; box-shadow: none; border-color: #21262d; }
  .card-name { font-weight: 600; }

  /* Enemies */
  .enemies { display: flex; gap: 8px; flex-wrap: wrap; }
  .enemy {
    background: #21262d; border: 1px solid #30363d;
    border-radius: 6px; padding: 8px 10px; min-width: 130px;
  }
  .enemy-header { font-weight: bold; color: #f85149; font-size: 0.82em; }
  .enemy-hp { color: #c9d1d9; font-size: 0.78em; margin: 2px 0; }
  .enemy-intent { color: #f0c050; font-size: 0.72em; }
  .hp-bar { height: 5px; background: #30363d; border-radius: 3px; margin-top: 3px; }
  .hp-fill { height: 100%; border-radius: 3px; transition: width 0.3s; }
  .hp-fill-red { background: linear-gradient(90deg, #da3633, #f85149); }
  .hp-fill-green { background: linear-gradient(90deg, #238636, #3fb950); }

  /* Actions */
  .action-section { margin-bottom: 10px; }
  .actions { display: flex; flex-wrap: wrap; gap: 5px; }
  .action-btn {
    background: #21262d; border: 1px solid #30363d;
    border-radius: 6px; padding: 5px 12px; font-size: 0.78em;
    color: #c9d1d9; cursor: pointer; transition: all 0.15s;
  }
  .action-btn:hover { background: #30363d; border-color: #58a6ff; }
  .action-btn.mcts-rec {
    border-color: #3fb950; box-shadow: 0 0 6px rgba(63,185,80,0.25);
  }
  .action-btn.mcts-rec::after { content: ' ★'; color: #3fb950; font-size: 0.9em; }

  .mcts-info { font-size: 0.72em; color: #8b949e; margin-top: 6px; line-height: 1.5; }
  .mcts-row { display: inline-block; margin-right: 10px; }
  .mcts-best-tag { color: #3fb950; font-weight: bold; }

  .log {
    background: #0d1117; border: 1px solid #30363d;
    border-radius: 6px; padding: 6px;
    max-height: 180px; overflow-y: auto; font-size: 0.72em;
    font-family: 'Consolas', monospace;
  }
  .log-entry { padding: 2px 0; border-bottom: 1px solid #161b22; }
  .log-entry:first-child { color: #58a6ff; }

  /* Collapsible panels */
  .info-panels { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-bottom: 10px; }
  .info-toggle { cursor: pointer; user-select: none; }
  .info-toggle::before { content: '▸ '; }
  .info-toggle.open::before { content: '▾ '; }
  .info-content { display: none; margin-top: 6px; }
  .info-content.open { display: block; }
  .pile-cards { display: flex; flex-wrap: wrap; gap: 3px; }
  .pile-tag {
    background: #21262d; border: 1px solid #30363d; border-radius: 4px;
    padding: 2px 6px; font-size: 0.68em; color: #8b949e;
  }
  .reward-item {
    background: #21262d; border: 1px solid #30363d; border-radius: 4px;
    padding: 4px 8px; font-size: 0.75em; margin: 2px 0;
  }
  .reward-gold { color: #d2a038; }
  .reward-card { color: #58a6ff; }
  .reward-relic { color: #f0c050; }
  .reward-potion { color: #3fb950; }
  .pile-info { font-size: 0.75em; color: #8b949e; margin-top: 6px; }
</style>
</head>
<body>
<div class="container">
  <h1>🃏 STS Simulator — Interactive Play</h1>
  <div class="subtitle">人工游玩 / AI观察 / 模拟器测试</div>

  <div class="controls">
    <button class="ctrl-btn" onclick="newGame()">🔄 New Game</button>
    <input id="seed-input" type="number" value="42" placeholder="Seed">
    <button class="ctrl-btn secondary" onclick="aiStep()">🤖 AI Step</button>
    <button class="ctrl-btn secondary" onclick="aiAutoPlay(10)">⏩ AI ×10</button>
    <button class="ctrl-btn secondary" onclick="aiAutoPlay(50)">⏭ AI ×50</button>
    <button class="ctrl-btn danger" onclick="resetGame()">🗑 Reset</button>
    <span id="timer" style="color:#8b949e;font-size:0.75em;margin-left:8px"></span>
  </div>

  <div class="status-bar" id="status-bar"></div>

  <div class="panels">
    <div class="panel">
      <h3>🃏 Hand</h3>
      <div class="hand" id="hand"></div>
      <div class="pile-info" id="pile-info"></div>
    </div>
    <div class="panel">
      <h3>👹 Enemies</h3>
      <div class="enemies" id="enemies"></div>
    </div>
  </div>

  <div class="panel action-section">
    <h3>⚡ Actions</h3>
    <div class="actions" id="actions"></div>
    <div class="mcts-info" id="mcts-info"></div>
  </div>

  <div class="panel">
    <h3>📜 Log</h3>
    <div class="log" id="log"></div>
  </div>

  <div class="info-panels">
    <div class="panel">
      <h3 class="info-toggle" onclick="togglePanel('deck-panel')">📋 Master Deck</h3>
      <div class="info-content" id="deck-panel"></div>
    </div>
    <div class="panel">
      <h3 class="info-toggle" onclick="togglePanel('rewards-panel')">🎁 Rewards</h3>
      <div class="info-content" id="rewards-panel"></div>
    </div>
    <div class="panel">
      <h3 class="info-toggle" onclick="togglePanel('draw-panel')">📥 Draw Pile</h3>
      <div class="info-content" id="draw-panel"></div>
    </div>
    <div class="panel">
      <h3 class="info-toggle" onclick="togglePanel('discard-panel')">📤 Discard Pile</h3>
      <div class="info-content" id="discard-panel"></div>
    </div>
  </div>
</div>

<script>
let currentState = null;

async function api(endpoint, data={}) {
  const r = await fetch('/api/' + endpoint, {
    method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify(data)
  });
  return r.json();
}

function getActionLabel(actionId, state) {
  // Dynamic labels using current hand card names
  if (actionId >= 0 && actionId <= 9 && state && state.hand && state.hand[actionId]) {
    const cost = state.hand_costs ? state.hand_costs[actionId] : '?';
    return `▶ ${state.hand[actionId]} [${cost}]`;
  }
  const STATIC = {
    10: '⏹ End Turn', 11: '🧪 Potion 0', 12: '🧪 Potion 1',
    13: '🧪 Potion 2', 14: '🧪 Potion 3',
    15: '🗑 Discard Potion 0', 16: '🗑 Discard Potion 1',
    17: '🗑 Discard Potion 2', 18: '🗑 Discard Potion 3',
    34: '💰 Take Gold', 35: '🏆 Take Relic', 36: '🧪 Take Potion',
    39: '⏭ Skip Rewards', 60: '💤 Rest', 61: '🔨 Smith',
    62: '💪 Lift', 63: '🚬 Toke', 64: '⛏ Dig', 65: '🔙 Recall',
    99: '➡️ Proceed'
  };
  if (STATIC[actionId]) return STATIC[actionId];
  if (actionId >= 20 && actionId <= 29) return `🗺 Map ${actionId - 20}`;
  if (actionId >= 30 && actionId <= 32) {
    // Show reward card names if available
    if (state && state.rewards) {
      const cardReward = state.rewards.find(r => r.startsWith('cards:'));
      if (cardReward) {
        const names = cardReward.replace('cards:', '').split(',');
        const idx = actionId - 30;
        if (idx < names.length) return `🃏 Pick: ${names[idx]}`;
      }
    }
    return `🃏 Card Reward ${actionId - 30}`;
  }
  if (actionId === 33) return `💰 Take Gold`;
  if (actionId === 34) return `🏆 Take Relic`;
  if (actionId === 35) return `🧪 Take Potion`;
  if (actionId >= 40 && actionId <= 44) return `🛒 Buy Card ${actionId - 40}`;
  if (actionId >= 45 && actionId <= 47) return `🛒 Buy Relic ${actionId - 45}`;
  if (actionId >= 48 && actionId <= 50) return `🛒 Buy Potion ${actionId - 48}`;
  if (actionId >= 90 && actionId <= 93) return `❓ Choice ${actionId - 89}`;
  return `Action ${actionId}`;
}

async function newGame() {
  const seed = parseInt(document.getElementById('seed-input').value) || 42;
  await api('new_game', {seed});
  document.getElementById('log').innerHTML = '';
  addLog('🎮 New game started (seed: ' + seed + ')');
  await refresh();
}
async function resetGame() { await newGame(); }

async function doAction(actionId) {
  const label = getActionLabel(actionId, currentState);
  const result = await api('step', {action: actionId});
  const rStr = result.reward >= 0 ? `+${result.reward.toFixed(1)}` : result.reward.toFixed(1);
  addLog(`#${result.step} ${label} → ${rStr}`);
  if (result.done) addLog('🏁 GAME OVER');
  await refresh();
}

async function aiStep() {
  const result = await api('ai_step');
  if (result.error) { addLog('⚠️ ' + result.error); return; }
  const label = getActionLabel(result.action, currentState);
  const rStr = result.reward >= 0 ? `+${result.reward.toFixed(1)}` : result.reward.toFixed(1);
  addLog(`🤖 #${result.step} ${label} → ${rStr}`);
  if (result.done) addLog('🏁 GAME OVER');
  await refresh();
}

async function aiAutoPlay(n) {
  for (let i = 0; i < n; i++) {
    await refresh();
    if (currentState && currentState.screen === 'GAME_OVER') break;
    const result = await api('ai_step');
    if (result.done || result.error) break;
  }
  await refresh();
  addLog(`🤖 AI played ${n} steps`);
}

function addLog(msg) {
  const log = document.getElementById('log');
  log.innerHTML = `<div class="log-entry">${msg}</div>` + log.innerHTML;
}

async function refresh() {
  const s = await api('get_state');
  currentState = s;

  // Status bar
  document.getElementById('status-bar').innerHTML = `
    <div class="stat"><div class="stat-label">Screen</div><span class="screen-tag">${s.screen||'?'}</span></div>
    <div class="stat"><div class="stat-label">HP</div><div class="stat-value hp-color">${s.hp}/${s.max_hp}</div></div>
    <div class="stat"><div class="stat-label">Energy</div><div class="stat-value energy-color">${s.energy||0}/${s.max_energy||0}</div></div>
    <div class="stat"><div class="stat-label">Block</div><div class="stat-value block-color">${s.block||0}</div></div>
    <div class="stat"><div class="stat-label">Gold</div><div class="stat-value gold-color">${s.gold||0}</div></div>
    <div class="stat"><div class="stat-label">Act</div><div class="stat-value">${s.act||1}</div></div>
    <div class="stat"><div class="stat-label">Floor</div><div class="stat-value">${s.floor||0}</div></div>
    <div class="stat"><div class="stat-label">Turn</div><div class="stat-value">${s.turn||0}</div></div>
    <div class="stat"><div class="stat-label">Step</div><div class="stat-value">${s.step||0}</div></div>
  `;

  // Hand
  const hand = document.getElementById('hand');
  if (s.hand && s.hand.length > 0) {
    hand.innerHTML = s.hand.map((name, i) => {
      const cost = s.hand_costs ? s.hand_costs[i] : '?';
      const playable = s.valid_actions && s.valid_actions.includes(i);
      return `<div class="card ${playable?'':'unplayable'}" onclick="${playable?`doAction(${i})`:''}" title="Action ${i}">
        <span class="card-name">${name}</span>
        <span class="cost-badge">${cost}</span>
      </div>`;
    }).join('');
  } else {
    hand.innerHTML = '<em style="color:#8b949e">No cards</em>';
  }

  // Pile info
  document.getElementById('pile-info').innerHTML =
    `📥 Draw: ${s.draw_pile_size||0} | 📤 Discard: ${s.discard_pile_size||0} | 🔥 Exhaust: ${s.exhaust_pile_size||0}`;

  // Enemies
  const enemies = document.getElementById('enemies');
  if (s.enemies && s.enemies.length > 0) {
    enemies.innerHTML = s.enemies.map((e, i) => {
      const pct = Math.max(0, Math.min(100, (e.hp / Math.max(e.max_hp, 1)) * 100));
      const color = pct > 50 ? 'hp-fill-red' : 'hp-fill-red';
      const intentStr = e.intent_dmg > 0 ? `${e.intent} (${e.intent_dmg}dmg)` : e.intent;
      return `<div class="enemy">
        <div class="enemy-header">Monster ${i+1}</div>
        <div class="enemy-hp">❤️ ${e.hp}/${e.max_hp}${e.block > 0 ? ` 🛡${e.block}` : ''}</div>
        <div class="hp-bar"><div class="hp-fill hp-fill-red" style="width:${pct}%"></div></div>
        <div class="enemy-intent">🎯 ${intentStr}</div>
      </div>`;
    }).join('');
  } else {
    enemies.innerHTML = '<em style="color:#8b949e">No enemies</em>';
  }

  // Actions — use dynamic card-name labels
  const actions = document.getElementById('actions');
  if (s.valid_actions && s.valid_actions.length > 0) {
    actions.innerHTML = s.valid_actions.map(a => {
      const isBest = s.mcts_best === a;
      const label = getActionLabel(a, s);
      return `<button class="action-btn ${isBest?'mcts-rec':''}" onclick="doAction(${a})">${label}</button>`;
    }).join('');
  } else {
    actions.innerHTML = '<em style="color:#8b949e">No valid actions</em>';
  }

  // MCTS info with card names
  const mctsInfo = document.getElementById('mcts-info');
  if (s.mcts_actions && s.mcts_actions.length > 0) {
    mctsInfo.innerHTML = '<strong>MCTS Analysis:</strong><br>' +
      s.mcts_actions.map(a => {
        const label = getActionLabel(a.action, s);
        const isBest = a.action === s.mcts_best;
        const cls = isBest ? 'mcts-best-tag' : '';
        const survPct = ((a.survival||0) * 100).toFixed(0);
        return `<span class="mcts-row ${cls}">${label}: HP=${(a.avg_hp||0).toFixed(1)}, surv=${survPct}%</span>`;
      }).join('');
  } else {
    mctsInfo.innerHTML = '';
  }

  // Info panels — always update content
  renderPilePanel('deck-panel', s.master_deck || [], '📋');
  renderPilePanel('draw-panel', s.draw_pile || [], '📥');
  renderPilePanel('discard-panel', s.discard_pile || [], '📤');
  renderRewardsPanel(s.rewards || []);
}

function togglePanel(id) {
  const el = document.getElementById(id);
  const toggle = el.previousElementSibling;
  el.classList.toggle('open');
  toggle.classList.toggle('open');
}

function renderPilePanel(id, cards, icon) {
  const el = document.getElementById(id);
  if (cards.length === 0) {
    el.innerHTML = '<em style="color:#8b949e">Empty</em>';
    return;
  }
  // Count duplicates
  const counts = {};
  cards.forEach(c => counts[c] = (counts[c]||0) + 1);
  const sorted = Object.entries(counts).sort((a,b) => b[1]-a[1]);
  el.innerHTML = `<div style="font-size:0.7em;color:#8b949e;margin-bottom:4px">${cards.length} cards</div>` +
    '<div class="pile-cards">' +
    sorted.map(([name, count]) =>
      `<span class="pile-tag">${name}${count > 1 ? ' ×'+count : ''}</span>`
    ).join('') + '</div>';
}

function renderRewardsPanel(rewards) {
  const el = document.getElementById('rewards-panel');
  if (rewards.length === 0) {
    el.innerHTML = '<em style="color:#8b949e">No rewards</em>';
    return;
  }
  el.innerHTML = rewards.map(r => {
    if (r.startsWith('gold:')) {
      return `<div class="reward-item reward-gold">💰 Gold: ${r.replace('gold:', '')}</div>`;
    } else if (r.startsWith('cards:')) {
      const cards = r.replace('cards:', '').split(',');
      return `<div class="reward-item reward-card">🃏 Cards: ${cards.join(', ')}</div>`;
    } else if (r.startsWith('relic:')) {
      return `<div class="reward-item reward-relic">🏆 Relic: ${r.replace('relic:', '')}</div>`;
    } else if (r.startsWith('potion:')) {
      return `<div class="reward-item reward-potion">🧪 Potion: ${r.replace('potion:', '')}</div>`;
    }
    return `<div class="reward-item">${r}</div>`;
  }).join('');
}

newGame();
</script>
</body>
</html>
"""

@app.route('/')
def index():
    return render_template_string(HTML_TEMPLATE)

@app.route('/api/new_game', methods=['POST'])
def api_new_game():
    data = request.json or {}
    init_game(data.get('seed', 42))
    return jsonify({'ok': True, 'seed': game['seed']})

@app.route('/api/get_state', methods=['POST'])
def api_get_state():
    return jsonify(get_game_state())

@app.route('/api/step', methods=['POST'])
def api_step():
    data = request.json or {}
    env = game['env']
    if env is None: return jsonify({'error': 'No game'})
    done, reward = env.step(data.get('action', 99))
    game['step_count'] += 1
    return jsonify({'done': done, 'reward': reward, 'step': game['step_count'],
                    'action': data.get('action', 99)})

@app.route('/api/ai_step', methods=['POST'])
def api_ai_step():
    env = game['env']
    if env is None: return jsonify({'error': 'No game'})
    screen = env.get_screen_type()
    if screen == 'GAME_OVER':
        return jsonify({'error': 'Game over', 'done': True})

    mask = env.get_valid_actions_mask()
    valid = [i for i, v in enumerate(mask) if v]
    if not valid:
        done, r = env.step(99)
        game['step_count'] += 1
        return jsonify({'action': 99, 'reward': r, 'done': done, 'step': game['step_count']})

    if screen == 'COMBAT':
        result = env.mcts_evaluate(n_sims=20, max_turns=10)
        action = result['best_action']
    else:
        action = pick_noncombat(valid, screen)

    done, reward = env.step(action)
    game['step_count'] += 1
    return jsonify({'action': action, 'reward': reward, 'done': done, 'step': game['step_count']})

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--port', type=int, default=5050)
    parser.add_argument('--seed', type=int, default=42)
    args = parser.parse_args()
    init_game(args.seed)
    print(f"STS Interactive Play UI → http://localhost:{args.port}")
    app.run(host='0.0.0.0', port=args.port, debug=False)
