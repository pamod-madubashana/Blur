const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const els = {
  pathValue: document.getElementById('path-value'),
  browse: document.getElementById('btn-browse'),
  launch: document.getElementById('btn-launch'),
  launchLabel: document.getElementById('btn-launch-label'),
  clear: document.getElementById('btn-clear'),
  console: document.getElementById('console'),
  badge: document.getElementById('status-badge'),
  statusLabel: document.getElementById('status-label'),
  gaugeFill: document.getElementById('gauge-fill'),
  gaugeStage: document.getElementById('gauge-stage'),
  gaugeSub: document.getElementById('gauge-sub'),
  gaugeTicks: document.getElementById('gauge-ticks'),
};

let gamePath = null;
let running = false;

const STAGES = {
  idle:       { label: 'STANDBY',    stage: 'READY',     sub: 'select game to begin', pct: 0.02,  color: 'var(--text-lo)' },
  disabling:  { label: 'DISABLING',  stage: 'OFFLINE',   sub: 'dropping adapters',    pct: 0.2,   color: 'var(--amber)' },
  waiting:    { label: 'SETTLING',   stage: 'STANDBY',   sub: 'network settling',     pct: 0.35,  color: 'var(--amber)' },
  launching:  { label: 'LAUNCHING',  stage: 'IGNITION',  sub: 'starting blur.exe',    pct: 0.55,  color: 'var(--cyan)' },
  racing:     { label: 'RACING',     stage: 'LAN MODE',  sub: 'session in progress',  pct: 0.9,   color: 'var(--cyan)' },
  restoring:  { label: 'RESTORING',  stage: 'RESTORE',   sub: 're-enabling network',  pct: 0.7,   color: 'var(--green)' },
};

const CIRC = 2 * Math.PI * 100; // r=100

function drawTicks() {
  const g = els.gaugeTicks;
  const cx = 120, cy = 120, rOuter = 100, rInner = 92;
  for (let i = 0; i < 24; i++) {
    const angle = (i / 24) * Math.PI * 2 - Math.PI / 2;
    const x1 = cx + rInner * Math.cos(angle);
    const y1 = cy + rInner * Math.sin(angle);
    const x2 = cx + rOuter * Math.cos(angle);
    const y2 = cy + rOuter * Math.sin(angle);
    const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    line.setAttribute('x1', x1); line.setAttribute('y1', y1);
    line.setAttribute('x2', x2); line.setAttribute('y2', y2);
    g.appendChild(line);
  }
}
drawTicks();

function setStatus(state) {
  const cfg = STAGES[state] || STAGES.idle;
  els.badge.dataset.state = state;
  els.statusLabel.textContent = cfg.label;
  els.gaugeStage.textContent = cfg.stage;
  els.gaugeSub.textContent = cfg.sub;
  els.gaugeFill.style.stroke = cfg.color;
  els.gaugeFill.style.strokeDashoffset = String(CIRC * (1 - cfg.pct));
}

function timestamp() {
  const d = new Date();
  return d.toTimeString().slice(0, 8);
}

function log(msg, cls = '') {
  els.console.querySelector('.console-empty')?.remove();
  const line = document.createElement('div');
  line.className = `log-line ${cls}`.trim();
  const ts = document.createElement('span');
  ts.className = 'ts';
  ts.textContent = timestamp();
  line.appendChild(ts);
  line.appendChild(document.createTextNode(msg));
  els.console.appendChild(line);
  els.console.scrollTop = els.console.scrollHeight;
}

function shortenPath(p) {
  if (!p) return 'No path selected';
  const parts = p.split(/[\\/]/);
  if (parts.length <= 3) return p;
  return `...${parts.slice(-3).join('\\')}`;
}

function setPath(p) {
  gamePath = p;
  els.pathValue.textContent = shortenPath(p);
  els.pathValue.title = p || '';
  els.launch.disabled = !p || running;
  els.launchLabel.textContent = p ? 'START LAN MODE' : 'SELECT BLUR.EXE';
}

function setRunning(isRunning) {
  running = isRunning;
  els.browse.disabled = isRunning;
  els.launch.disabled = isRunning || !gamePath;
  els.launch.classList.toggle('running', isRunning);
  els.launchLabel.textContent = isRunning ? 'IN PROGRESS...' : (gamePath ? 'START LAN MODE' : 'SELECT BLUR.EXE');
}

async function init() {
  setStatus('idle');
  const saved = await invoke('get_saved_path');
  if (saved) setPath(saved);

  els.browse.addEventListener('click', async () => {
    const picked = await invoke('pick_game_path');
    if (picked) {
      setPath(picked);
      log(`Selected game: ${picked}`, 'system');
    }
  });

  els.launch.addEventListener('click', async () => {
    if (!gamePath || running) return;
    setRunning(true);
    try {
      await invoke('start_lan_mode', { gamePath });
    } catch (e) {
      log(`ERROR: ${e}`, 'error');
      setRunning(false);
      setStatus('idle');
    }
  });

  els.clear.addEventListener('click', () => {
    els.console.innerHTML = '<div class="console-empty">Log cleared. Ready for next run.</div>';
  });

  await listen('log', (event) => {
    const msg = event.payload;
    log(msg, msg.startsWith('ERROR') ? 'error' : (msg.startsWith('===') ? 'system' : ''));
  });

  await listen('status', (event) => {
    setStatus(event.payload);
  });

  await listen('finished', () => {
    setRunning(false);
  });
}

init();
