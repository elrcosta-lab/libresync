// Raw Tauri IPC v2 (works without bundler, uses __TAURI_INTERNALS__)
let _callId = 0;
const _pending = {};

window.addEventListener('message', (e) => {
  const { callback, error, data } = e.data || {};
  if (callback && _pending[callback]) {
    _pending[callback](data);
    delete _pending[callback];
  }
  if (error && _pending[error]) {
    _pending[error](data);
    delete _pending[error];
  }
});

async function invoke(cmd, args = {}) {
  // Tauri v2 public API (window.__TAURI__.core.invoke)
  const tauri = window.__TAURI__;
  if (tauri && tauri.core && typeof tauri.core.invoke === 'function') {
    return tauri.core.invoke(cmd, args);
  }

  // Fallback: raw Tauri internals (for older or non-bundler builds)
  const internals = window.__TAURI_INTERNALS__;
  if (!internals || !internals.postMessage) {
    throw new Error('Tauri IPC não disponível');
  }
  return new Promise((resolve, reject) => {
    const cid = ++_callId;
    const callback = `_${cid}`;
    const error = `_e${cid}`;
    _pending[callback] = resolve;
    _pending[error] = reject;
    internals.postMessage({
      cmd: 'invoke',
      callback,
      error,
      message: { cmd, args }
    });
    // Timeout de 5s — se backend não responder, rejeita
    setTimeout(() => {
      delete _pending[callback];
      delete _pending[error];
      reject(new Error(`IPC timeout: ${cmd}`));
    }, 5000);
  });
}

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

function showLoading(show) {
  document.getElementById('loading-overlay').classList.toggle('hidden', !show);
}

function showNotification(msg, type) {
  const el = document.getElementById('notification');
  el.textContent = msg;
  el.className = 'notification ' + (type || 'info');
  el.classList.remove('hidden');
  setTimeout(() => el.classList.add('hidden'), 5000);
}

function showScreen(name) {
  $$('.screen').forEach(el => el.classList.add('hidden'));
  const screen = $(`#screen-${name}`);
  if (screen) screen.classList.remove('hidden');
}

function showLogin() {
  showScreen('login');
}

function showDashboard() {
  showScreen('dashboard');
}

function showSettings() {
  showScreen('settings');
  document.getElementById('settings-feedback')?.classList.add('hidden');
}

function showWelcome() {
  showScreen('welcome');
}

async function syncScreenFromBackend() {
  try {
    const state = await getState();
    const screen = getScreenName(state.screen);
    if (screen === 'Onboarding') {
      showWelcome();
      return;
    } else if (screen === 'Preferences') {
      showSettings();
      return;
    } else if (screen === 'Main') {
      if (state.active_account || (state.accounts && state.accounts.length > 0)) {
        showDashboard();
      } else {
        showLogin();
      }
      return;
    }
  } catch (e) {
    console.warn('syncScreenFromBackend falhou:', e);
  }
}

// --- LOGIN ---

async function loadAccounts() {
  try {
    const state = await getState();
    const section = $('#accounts-section');
    const list = $('#accounts-list');
    const accounts = state.accounts || [];
    if (accounts.length > 0) {
      section.classList.remove('hidden');
      list.innerHTML = accounts.map(acc => `
        <li>
          <span>${acc.email || acc.id}</span>
          <button class="account-remove" onclick="removeAccount('${acc.id}')">✕ Remover</button>
        </li>
      `).join('');
    } else {
      section.classList.add('hidden');
    }
  } catch (e) {
    console.error('loadAccounts:', e);
  }
}

async function removeAccount(id) {
  showLoading(true);
  try {
    await logout(id);
    await loadAccounts();
  } catch (e) {
    console.error('removeAccount:', e);
  }
  showLoading(false);
}

// --- DASHBOARD ---

let dashboardInterval = null;

function startPolling() {
  stopPolling();
  dashboardInterval = setInterval(refreshDashboard, 5000);
}

function stopPolling() {
  if (dashboardInterval) {
    clearInterval(dashboardInterval);
    dashboardInterval = null;
  }
}

async function refreshDashboard() {
  try {
    const state = await getState();
    updateStatus(state.status);
    updateAccount(state);
    updateActivity(state.recent_activity);
    const pauseBtn = $('#pause-btn');
    if (state.status === 'paused') {
      pauseBtn.innerHTML = '▶️ Resume';
      pauseBtn.className = 'btn btn-secondary';
    } else {
      pauseBtn.innerHTML = '⏸️ Pausar';
      pauseBtn.className = 'btn';
    }
  } catch (e) {
    updateStatus('offline');
    console.error('refreshDashboard:', e);
  }
}

function updateStatus(status) {
  const badge = $('#status-badge');
  const map = {
    synced: { cls: 'status-synced', label: '✓ Sincronizado' },
    syncing: { cls: 'status-syncing', label: '🔄 Sincronizando' },
    paused: { cls: 'status-paused', label: '⏸️ Pausado' },
    error: { cls: 'status-error', label: '⚠️ Erro' },
    offline: { cls: 'status-offline', label: '○ Offline' },
  };
  const s = map[status] || map.offline;
  badge.className = `status-badge ${s.cls}`;
  badge.textContent = s.label;
}

function updateAccount(state) {
  const info = $('#account-info');
  const quota = $('#quota-info');
  const fill = quota.querySelector('.quota-fill');
  const text = quota.querySelector('.quota-text');

  if (state.active_account) {
    info.textContent = state.active_account.email || state.active_account.id;
    const used = state.active_account.quota_used || 0;
    const total = state.active_account.quota_total || 1;
    const pct = Math.min((used / total) * 100, 100);
    fill.style.width = pct + '%';
    text.textContent = formatBytes(used) + ' / ' + formatBytes(total);
  } else {
    info.textContent = 'Nenhuma conta ativa';
    fill.style.width = '0%';
    text.textContent = '—';
  }
}

function updateActivity(events) {
  const list = $('#activity-list');
  if (!events || events.length === 0) {
    list.innerHTML = '<li class="activity-empty">Nenhuma atividade ainda</li>';
    return;
  }
  list.innerHTML = events.slice(0, 20).map(ev => {
    const iconMap = { created: '📄', modified: '✏️', deleted: '🗑️', renamed: '📝' };
    const icon = iconMap[ev.type] || '📄';
    const time = ev.timestamp ? formatTime(ev.timestamp) : '';
    return `
      <li>
        <span class="activity-icon">${icon}</span>
        <span class="activity-file">${escapeHtml(ev.file || ev.path || '')}</span>
        <span class="activity-time">${time}</span>
      </li>
    `;
  }).join('');
}

async function togglePause() {
  showLoading(true);
  try {
    await togglePauseCmd();
    await refreshDashboard();
  } catch (e) {
    console.error('togglePause:', e);
  }
  showLoading(false);
}

async function logout() {
  showLoading(true);
  try {
    const state = await getState();
    if (state.active_account) {
      await logout(state.active_account.id);
    }
    stopPolling();
    showLogin();
  } catch (e) {
    console.error('logout:', e);
  }
  showLoading(false);
}

// --- SETTINGS ---

async function loadSettings() {
  try {
    const s = await getSettingsCmd();
    $('#settings-sync-folder').value = s.sync_folder || '';
    $('#settings-client-id').value = s.client_id || '';
    $('#settings-client-secret').value = s.client_secret || '';
    $('#settings-bandwidth').value = s.bandwidth_limit || '';
    $('#settings-autostart').checked = s.auto_start || false;
    $('#settings-polling').value = s.polling_interval || 30;
  } catch (e) {
    console.error('loadSettings:', e);
  }
}

async function saveSettings() {
  const fb = $('#settings-feedback');
  fb.classList.add('hidden');
  const settings = {
    sync_folder: $('#settings-sync-folder').value,
    client_id: $('#settings-client-id').value,
    client_secret: $('#settings-client-secret').value,
    bandwidth_limit: parseInt($('#settings-bandwidth').value) || 0,
    auto_start: $('#settings-autostart').checked,
    polling_interval: parseInt($('#settings-polling').value) || 30,
  };
  // Save locally for tray menu to use
  localStorage.setItem('libresync_settings', JSON.stringify(settings));
  try {
    await updateSettingsCmd(settings);
    fb.textContent = '✅ Salvo!';
    fb.className = 'feedback success';
    fb.classList.remove('hidden');
    setTimeout(() => fb.classList.add('hidden'), 3000);
  } catch (e) {
    fb.textContent = '✅ Salvo (local)';
    fb.className = 'feedback success';
    fb.classList.remove('hidden');
    setTimeout(() => fb.classList.add('hidden'), 3000);
    console.error('saveSettings (IPC failed):', e);
  }
}

function selectFolder() {
  console.warn('Folder selection via native dialog not yet implemented');
}

// --- Tauri IPC wrappers ---

async function getState() {
  return await invoke('get_state');
}

async function togglePauseCmd() {
  return await invoke('toggle_pause');
}

async function getActivity(limit) {
  return await invoke('get_activity', { limit });
}

async function updateSettingsCmd(settings) {
  return await invoke('update_settings', { settings });
}

async function getSettingsCmd() {
  return await invoke('get_settings');
}

async function completeWelcome() {
  showLoading(true);
  try {
    const clientId = document.getElementById('welcome-client-id').value.trim();
    const clientSecret = document.getElementById('welcome-client-secret').value.trim();
    await invoke('complete_welcome', { clientId, clientSecret });
    const state = await getState();
    if (state.active_account) {
      showDashboard();
      startPolling();
    } else if (state.accounts && state.accounts.length > 0) {
      showDashboard();
    } else {
      showLogin();
    }
    showNotification('Configuração salva! Use o tray → Conectar conta Google.', 'success');
  } catch (e) {
    showNotification('Erro ao salvar: ' + e, 'error');
  }
  showLoading(false);
}

function openUrl(url) {
  window.open(url, '_blank');
}

async function login() {
  showLoading(true);
  try {
    const url = await invoke('login');
    const state = await getState();
    if (state.active_account) {
      showDashboard();
      startPolling();
    } else {
      await loadAccounts();
    }
  } catch (e) {
    showNotification(e, 'error');
  }
  showLoading(false);
}

async function logout(accountId) {
  return await invoke('logout', { accountId });
}

// --- helpers ---

function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const val = bytes / Math.pow(1024, i);
  return val.toFixed(i > 0 ? 1 : 0) + ' ' + units[i];
}

function formatTime(ts) {
  if (!ts) return '';
  const d = new Date(ts);
  if (isNaN(d.getTime())) return ts;
  return d.toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' });
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// --- INIT ---

function getScreenName(screen) {
  // serde serializa unit variants como string ("Login")
  // e struct variants como objeto ({"Onboarding": {"step": 1}})
  if (typeof screen === 'string') return screen;
  if (screen && typeof screen === 'object') return Object.keys(screen)[0] || '';
  return '';
}

document.addEventListener('DOMContentLoaded', async () => {
  await syncScreenFromBackend();
  // Fallback: se o backend não expõe screen ou getState falha, mostra login
  if (document.querySelectorAll('.screen:not(.hidden)').length === 0) {
    showLogin();
  }
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') {
      syncScreenFromBackend();
    }
  });
});
