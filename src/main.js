const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const els = {
  launch: document.getElementById('btn-launch'),
  launchLabel: document.getElementById('btn-launch-label'),
  console: document.getElementById('console'),
  badge: document.getElementById('status-badge'),
  statusLabel: document.getElementById('status-label'),
  adapterList: document.getElementById('adapter-list'),
};

let gamePath = null;
let running = false;
let adapterPollInterval = null;
let refreshingAdapters = false;

// ===== STATUS LABELS =====
const STATUS_LABELS = {
  idle:      'STANDBY',
  disabling: 'DISABLING',
  waiting:   'SETTLING',
  launching: 'LAUNCHING',
  racing:    'RACING',
  restoring: 'RESTORING',
};

// ===== ADAPTER ICONS (SVG paths) =====
const ICONS = {
  wifi: '<path d="M5 12.55a11 11 0 0114 0M8.53 16.11a6 6 0 016.95 0M12 20h.01"/>',
  ethernet: '<path d="M22 12h-4l-3 9L9 3l-3 9H2"/>',
  virtual: '<rect x="2" y="2" width="20" height="20" rx="2" ry="2"/><path d="M7 12h10M12 7v10"/>',
};

function adapterIcon(type) {
  const path = ICONS[type] || ICONS.ethernet;
  return `<svg class="adapter-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${path}</svg>`;
}

// ===== STATUS =====
function setStatus(state) {
  const label = STATUS_LABELS[state] || 'STANDBY';
  els.badge.dataset.state = state;
  els.statusLabel.textContent = label;
}

// ===== LOG =====
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
  ts.textContent = `[${timestamp()}]`;
  const msgEl = document.createElement('span');
  msgEl.className = 'msg';

  if (msg.startsWith('  Copied:')) {
    msgEl.innerHTML = `<span class="copied">${escapeHtml(msg)}</span>`;
  } else {
    msgEl.textContent = msg;
  }

  line.appendChild(ts);
  line.appendChild(msgEl);
  els.console.appendChild(line);
  els.console.scrollTop = els.console.scrollHeight;
}

function escapeHtml(str) {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// ===== RUNNING STATE =====
function setRunning(isRunning) {
  running = isRunning;
  els.launch.disabled = isRunning;
  els.launch.classList.toggle('running', isRunning);
  els.launchLabel.textContent = isRunning ? 'STOP' : 'START';

  // Poll adapters only during enable/disable to show real-time changes
  if (isRunning) {
    startAdapterPoll();
  } else {
    stopAdapterPoll();
  }
}

// ===== ADAPTERS =====
function statusLabel(status, type) {
  // Only WiFi (internet-connected) shows ONLINE
  if (status === 'Up' && type === 'wifi') return 'ONLINE';
  // All others: just enabled or disabled
  if (status === 'Disabled' || status === 'Not Present') return 'DISABLED';
  return 'ENABLED';
}

function isLocked(name, type) {
  return type === 'wifi' && running;
}

async function refreshAdapters() {
  // Prevent concurrent calls from stacking
  if (refreshingAdapters) return;
  refreshingAdapters = true;
  try {
    const adapters = await invoke('list_adapters');
    renderAdapters(adapters);
  } catch (e) {
    // Silently fail during polling
  } finally {
    refreshingAdapters = false;
  }
}

function renderAdapters(adapters) {
  if (!adapters || adapters.length === 0) {
    els.adapterList.innerHTML = '<div class="adapter-empty">No adapters found</div>';
    return;
  }

  const statusOrder = { 'Up': 0, 'Disabling': 1, 'Enabling': 2, 'Disabled': 3, 'Not Present': 4 };
  adapters.sort((a, b) => {
    const ta = a.adapter_type === 'wifi' ? -1 : 0;
    const tb = b.adapter_type === 'wifi' ? -1 : 0;
    if (ta !== tb) return ta - tb;
    return (statusOrder[a.status] ?? 5) - (statusOrder[b.status] ?? 5);
  });

  els.adapterList.innerHTML = adapters.map(a => {
    const locked = isLocked(a.name, a.adapter_type);
    const isDisabling = a.status === 'Disabling';
    const isOffline = a.status === 'Disabled' || a.status === 'Not Present';
    const classes = [
      'adapter-item',
      isDisabling ? 'disabling' : '',
      isOffline ? 'offline' : '',
    ].filter(Boolean).join(' ');

    return `<div class="${classes}" data-adapter-status="${a.status}" data-adapter-type="${a.adapter_type}" data-adapter-locked="${locked}">
      <div class="adapter-left">
        ${adapterIcon(a.adapter_type)}
        <span class="adapter-name">${escapeHtml(a.name)}</span>
      </div>
      <div class="adapter-right">
        <span class="adapter-status">${locked ? 'LOCKED' : statusLabel(a.status, a.adapter_type)}</span>
        <div class="adapter-dot"></div>
      </div>
    </div>`;
  }).join('');
}

function startAdapterPoll() {
  if (adapterPollInterval) return;
  refreshAdapters();
  adapterPollInterval = setInterval(refreshAdapters, 2000);
}

function stopAdapterPoll() {
  if (adapterPollInterval) {
    clearInterval(adapterPollInterval);
    adapterPollInterval = null;
  }
}

// ===== INIT =====
async function init() {
  setStatus('idle');

  // Silently load saved path (no prompt on startup)
  const saved = await invoke('get_saved_path');
  if (saved) {
    gamePath = saved;
    log(`Game path loaded: ${saved}`, 'system');
  }

  // Initial adapter scan (one-time, no polling)
  await refreshAdapters();

  // START / STOP button
  els.launch.addEventListener('click', async () => {
    if (running) return;

    // Check for game path; prompt file picker if missing
    if (!gamePath) {
      log('No saved game path. Opening file picker...', 'system');
      const picked = await invoke('pick_game_path');
      if (picked) {
        gamePath = picked;
        log(`Game selected: ${picked}`, 'system');
      } else {
        log('No game selected. Aborting launch.', 'error');
        return;
      }
    }

    setRunning(true);
    try {
      await invoke('start_lan_mode', { gamePath });
    } catch (e) {
      log(`ERROR: ${e}`, 'error');
      setRunning(false);
      setStatus('idle');
    }
  });

  // Listen for events from Rust backend
  await listen('log', (event) => {
    const msg = event.payload;
    if (msg.startsWith('ERROR')) {
      log(msg, 'error');
    } else if (msg.startsWith('===')) {
      log(msg, 'system');
    } else {
      log(msg);
    }
  });

  await listen('status', (event) => {
    setStatus(event.payload);
  });

  await listen('finished', () => {
    setRunning(false);
    setStatus('idle');
    // Final refresh after adapters restored
    refreshAdapters();
  });
}

init();
