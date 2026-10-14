const commands = [
  { name: 'play', path: '/cmd/ytmd/play', methods: ['POST'], description: 'Change the player state to play' },
  { name: 'pause', path: '/cmd/ytmd/pause', methods: ['POST'], description: 'Change the player state to pause' },
  { name: 'toggle-play', path: '/cmd/ytmd/toggle-play', methods: ['POST'], description: 'Toggle play/pause' },
  { name: 'previous', path: '/cmd/ytmd/previous', methods: ['POST'], description: 'Play the previous song' },
  { name: 'next', path: '/cmd/ytmd/next', methods: ['POST'], description: 'Play the next song' },
  { name: 'like-state', path: '/cmd/ytmd/like-state', methods: ['GET'], description: 'Get the current like state' },
  { name: 'like', path: '/cmd/ytmd/like', methods: ['POST'], description: 'Set the current song as liked' },
  { name: 'dislike', path: '/cmd/ytmd/dislike', methods: ['POST'], description: 'Set the current song as disliked' },
  { name: 'seek-to', path: '/cmd/ytmd/seek-to', methods: ['POST'], description: 'Seek to a specific time' },
  { name: 'go-back', path: '/cmd/ytmd/go-back', methods: ['POST'], description: 'Move the current song backwards' },
  { name: 'go-forward', path: '/cmd/ytmd/go-forward', methods: ['POST'], description: 'Move the current song forwards' },
  { name: 'shuffle', path: '/cmd/ytmd/shuffle', methods: ['GET', 'POST'], description: 'Get or change shuffle state' },
  { name: 'repeat-mode', path: '/cmd/ytmd/repeat-mode', methods: ['GET'], description: 'Get the current repeat mode' },
  { name: 'switch-repeat', path: '/cmd/ytmd/switch-repeat', methods: ['POST'], description: 'Switch repeat mode' },
  { name: 'volume', path: '/cmd/ytmd/volume', methods: ['GET', 'POST'], description: 'Get or set the player volume' },
  { name: 'fullscreen', path: '/cmd/ytmd/fullscreen', methods: ['GET', 'POST'], description: 'Get or set fullscreen mode' },
  { name: 'toggle-mute', path: '/cmd/ytmd/toggle-mute', methods: ['POST'], description: 'Toggle mute state' },
  { name: 'song', path: '/cmd/ytmd/song', methods: ['GET'], description: 'Get the current song details' },
  { name: 'queue', path: '/cmd/ytmd/queue', methods: ['GET', 'POST', 'PATCH', 'DELETE'], description: 'Get queue info or modify the queue' },
  { name: 'queue-next', path: '/cmd/ytmd/queue/next', methods: ['GET'], description: 'Get information about the next song in the queue' },
  { name: 'queue-index', path: '/cmd/ytmd/queue/{index}', methods: ['PATCH', 'DELETE'], description: 'Move or remove a queue item by index' },
  { name: 'search', path: '/cmd/ytmd/search', methods: ['POST'], description: 'Search for a song' },
  { name: 'chat-commands', path: '/cmd/jb/commands', methods: ['GET'], description: 'Twitch chat commands on/off status' },
  { name: 'chat-commands-on', path: '/cmd/jb/commands/on', methods: ['GET'], description: 'Enable Twitch chat commands' },
  { name: 'chat-commands-off', path: '/cmd/jb/commands/off', methods: ['GET'], description: 'Disable Twitch chat commands' },
];

const CHAT_COMMAND_ROWS = [
  { id: 'sr', aliases: '!sr / !songrequest' },
  { id: 'q', aliases: '!q / !queue' },
  { id: 'si', aliases: '!si / !songinfo' },
];

const select = document.getElementById('commandSelect');
const output = document.getElementById('commandOutput');
const argInput = document.getElementById('argInput');
const runButton = document.getElementById('runCommand');
const urlInput = document.getElementById('commandUrl');
const copyButton = document.getElementById('copyUrl');
const tabs = document.querySelectorAll('.tab');
const pages = document.querySelectorAll('.page');
const queueList = document.getElementById('queueList');
const queueCount = document.getElementById('queueCount');
const clearQueueButton = document.getElementById('clearQueueButton');
const previousToggle = document.getElementById('previousToggle');
const blacklistInput = document.getElementById('blacklistInput');
const addBlacklistButton = document.getElementById('addBlacklistButton');
const blacklistList = document.getElementById('blacklistList');
const settingsOutput = document.getElementById('settingsOutput');
const maxDurationInput = document.getElementById('maxDurationInput');
const saveMaxDurationButton = document.getElementById('saveMaxDurationButton');
const currentMeta = document.getElementById('currentMeta');
const playState = document.getElementById('playState');
const currentTime = document.getElementById('currentTime');
const totalTime = document.getElementById('totalTime');
const progressBar = document.getElementById('progressBar');
const currentHeadline = currentMeta?.parentElement || null;
const playerButtons = {
  prev: document.getElementById('prevButton'),
  back: document.getElementById('backButton'),
  play: document.getElementById('playButton'),
  pause: document.getElementById('pauseButton'),
  forward: document.getElementById('forwardButton'),
  next: document.getElementById('nextButton'),
  shuffle: document.getElementById('shuffleButton'),
};
const dockEl = document.querySelector('.dock');
const dockFold = document.getElementById('dockFold');
const compactPrev = document.getElementById('compactPrev');
const compactPlay = document.getElementById('compactPlay');
const compactNext = document.getElementById('compactNext');
const DOCK_FOLD_KEY = 'jb2-admin-dock-folded';
const songRequestInput = document.getElementById('songRequestInput');
const songRequestForm = document.getElementById('songRequestForm');
const songRequestNow = document.getElementById('songRequestNow');
const songSearchPopup = document.getElementById('songSearchPopup');
const seedStatusLine = document.getElementById('seedStatusLine');
const seedPlaylistInput = document.getElementById('seedPlaylistInput');
const addSeedPlaylistButton = document.getElementById('addSeedPlaylistButton');
const seedInputFeedback = document.getElementById('seedInputFeedback');
const clearQueueOnRequestCheckbox = document.getElementById('clearQueueOnRequestCheckbox');
const seedPlaylistList = document.getElementById('seedPlaylistList');
const twitchStatusLine = document.getElementById('twitchStatusLine');
const twitchChannelInput = document.getElementById('twitchChannelInput');
const twitchNickInput = document.getElementById('twitchNickInput');
const twitchSaveButton = document.getElementById('twitchSaveButton');
const twitchLoginButton = document.getElementById('twitchLoginButton');
const twitchConnectButton = document.getElementById('twitchConnectButton');
const twitchDisconnectButton = document.getElementById('twitchDisconnectButton');
const twitchFormFeedback = document.getElementById('twitchFormFeedback');
const twitchChatLog = document.getElementById('twitchChatLog');
const twitchChatForm = document.getElementById('twitchChatForm');
const twitchChatInput = document.getElementById('twitchChatInput');
const chatFormFeedback = document.getElementById('chatFormFeedback');
const chatCmdsPill = document.getElementById('chatCmdsPill');
const chatCmdsEnabled = document.getElementById('chatCmdsEnabled');
const cmdCooldownRows = document.getElementById('cmdCooldownRows');
const saveCmdCooldowns = document.getElementById('saveCmdCooldowns');
const cmdFeedback = document.getElementById('cmdFeedback');
const cmdHistoryLog = document.getElementById('cmdHistoryLog');
const importFileInput = document.getElementById('importFileInput');
const importConfigButton = document.getElementById('importConfigButton');
const importFeedback = document.getElementById('importFeedback');
const aboutVersion = document.getElementById('aboutVersion');

let adminEvents = null;
let twitchStickBottom = true;
let twitchLastChatKey = null;

function updateSelection(selected) {
  if (!selected) return;

  // YTMD proxy encodes method as a path suffix; /cmd/jb/* is exact-match.
  const defaultMethod = selected.methods[0]?.toLowerCase() || 'get';
  const methodPath = selected.path.startsWith('/cmd/ytmd')
    ? `${selected.path}/${defaultMethod}`
    : selected.path;
  const fullUrl = `${window.location.origin}${methodPath}`;
  urlInput.value = fullUrl;
  output.textContent = `${selected.methods.join(', ')} ${methodPath}\n\n${selected.description}`;
}

function renderCommands() {
  select.innerHTML = '';

  commands.forEach((cmd) => {
    const option = document.createElement('option');
    option.value = cmd.name;
    option.textContent = `${cmd.name} (${cmd.methods.join(', ')})`;
    select.appendChild(option);
  });

  select.addEventListener('change', () => {
    const selected = commands.find((cmd) => cmd.name === select.value);
    updateSelection(selected);
  });

  if (commands.length > 0) {
    const first = commands[0];
    select.value = first.name;
    updateSelection(first);
  }
}

async function runCommand() {
  const selected = commands.find((cmd) => cmd.name === select.value);
  if (!selected) return;

  const method = selected.methods[0];
  output.textContent = 'Running…';

  try {
    const body = argInput.value.trim();
    const init = {
      method,
      headers: { 'Content-Type': 'application/json' },
    };

    if (method !== 'GET' && body) {
      const parsed = body.startsWith('{') ? JSON.parse(body) : { value: body };
      init.body = JSON.stringify(parsed);
    }

    const response = await fetch(selected.path, init);
    const text = await response.text();

    let formattedText = text;
    try {
      const parsed = JSON.parse(text);
      formattedText = JSON.stringify(parsed, null, 2);
    } catch (_) {
      formattedText = text || `Status ${response.status}`;
    }

    output.textContent = formattedText;
    if (!response.ok) {
      output.textContent = `Request failed with status ${response.status}\n\n${formattedText}`;
    }
  } catch (err) {
    output.textContent = `Request failed: ${err.message}`;
  }
}

function activeAdminPage() {
  return document.querySelector('.tab.active')?.dataset.page || 'queue';
}

function showAdminPage(page) {
  tabs.forEach((item) => item.classList.toggle('active', item.dataset.page === page));
  pages.forEach((el) => el.classList.toggle('active', el.id === `page-${page}`));
  if (page === 'queue') {
    refreshQueueFromProxy();
  }
  if (page === 'seed') {
    loadSeedPanel();
  }
  if (page === 'twitch') {
    loadTwitchPanel();
  } else if (page === 'chat') {
    loadTwitchLive();
  }
  if (page === 'commands') {
    loadCommandsPanel();
  }
  if (page === 'about') {
    loadAboutPanel();
  }
}

tabs.forEach((tab) => {
  tab.addEventListener('click', () => {
    showAdminPage(tab.dataset.page);
  });
});

async function loadSeedPanel() {
  try {
    const response = await fetch('/api/seed/status');
    const payload = await readCommandEnvelope(response);
    renderSeedPanel(payload.data || {});
  } catch (err) {
    if (seedStatusLine) {
      seedStatusLine.textContent = `Seed mode: error (${err.message})`;
    }
  }
}

function renderSeedPanel(data) {
  if (seedStatusLine) {
    if (data.seedModeActive) {
      const requests = Number(data.activeRequestCount) || 0;
      const requestLabel = requests > 0
        ? `${requests} request${requests === 1 ? '' : 's'} waiting`
        : 'no requests waiting';
      let playingLabel = 'idle';
      if (data.playingKind === 'request') {
        playingLabel = 'playing a request';
      } else if (data.playingKind === 'seed') {
        playingLabel = 'playing seed';
      } else if (data.playingKind === 'other') {
        playingLabel = 'playing other track';
      }
      seedStatusLine.textContent = `Jukebox mode: Active · ${playingLabel} · ${requestLabel}`;
    } else {
      seedStatusLine.textContent = 'Jukebox mode: Inactive (seed fallback off)';
    }
    seedStatusLine.classList.toggle('seed-status--active', Boolean(data.seedModeActive));
  }
  if (clearQueueOnRequestCheckbox) {
    clearQueueOnRequestCheckbox.checked = Boolean(data.clearQueueOnRequest);
  }
  renderSeedPlaylists(data.seedPlaylists || []);
}

function renderSeedPlaylists(playlists) {
  if (!seedPlaylistList) return;
  seedPlaylistList.innerHTML = '';
  if (!playlists.length) {
    const item = document.createElement('li');
    item.className = 'seed-list__empty';
    item.textContent = 'No seed playlists saved yet';
    seedPlaylistList.appendChild(item);
    return;
  }

  playlists.forEach((playlist) => {
    const item = document.createElement('li');
    item.className = 'seed-list__item';

    const label = document.createElement('span');
    label.className = 'seed-list__label';
    const displayName = playlist.name || playlist.id || '';
    const labelText = `${playlist.trackCount || 0} - ${displayName}`;
    label.textContent = labelText;
    label.title = playlist.id && playlist.name && playlist.name !== playlist.id
      ? `${labelText} (${playlist.id})`
      : labelText;

    const playButton = document.createElement('button');
    playButton.type = 'button';
    playButton.className = 'queue-action-btn queue-action-btn--play';
    playButton.innerHTML = icon('player-play');
    playButton.setAttribute('aria-label', 'Enqueue seed playlist');
    playButton.addEventListener('click', () => enqueueSeedPlaylist(playlist.id));

    const purgeButton = document.createElement('button');
    purgeButton.type = 'button';
    purgeButton.className = 'seed-list__action';
    purgeButton.textContent = 'Purge';
    purgeButton.addEventListener('click', () => removeSeedTracksFromQueue(playlist.id));

    const removeButton = document.createElement('button');
    removeButton.type = 'button';
    removeButton.className = 'queue-action-btn queue-action-btn--delete';
    removeButton.innerHTML = icon('x');
    removeButton.setAttribute('aria-label', 'Remove playlist');
    removeButton.addEventListener('click', () => removeSeedPlaylist(playlist.id));

    const actions = document.createElement('div');
    actions.className = 'seed-list__actions';
    actions.append(purgeButton, playButton, removeButton);

    item.append(label, actions);
    seedPlaylistList.appendChild(item);
  });
}

async function addSeedPlaylist() {
  if (!seedPlaylistInput) return;
  const input = seedPlaylistInput.value.trim();
  if (!input) return;
  if (seedInputFeedback) seedInputFeedback.textContent = 'Adding playlist…';
  try {
    const response = await fetch('/api/seed/playlists', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: JSON.stringify({ input }),
    });
    const payload = await readCommandEnvelope(response);
    seedPlaylistInput.value = '';
    if (seedInputFeedback) seedInputFeedback.textContent = payload.message || 'Playlist saved';
    await loadSeedPanel();
    refreshQueueFromProxy();
  } catch (err) {
    if (seedInputFeedback) seedInputFeedback.textContent = err.message;
  }
}

async function enqueueSeedPlaylist(playlistId) {
  if (seedInputFeedback) seedInputFeedback.textContent = 'Adding tracks to queue…';
  try {
    const response = await fetch(`/cmd/jb/seed/enqueue?playlistId=${encodeURIComponent(playlistId)}`, {
      method: 'POST',
      headers: { Accept: 'application/json' },
    });
    const payload = await readCommandEnvelope(response);
    if (seedInputFeedback) seedInputFeedback.textContent = payload.message || 'Playlist queued';
    await loadSeedPanel();
    refreshQueueFromProxy();
  } catch (err) {
    if (seedInputFeedback) seedInputFeedback.textContent = err.message;
  }
}

async function removeSeedTracksFromQueue(playlistId) {
  try {
    const response = await fetch(`/cmd/jb/seed/queue?playlistId=${encodeURIComponent(playlistId)}`, {
      method: 'DELETE',
      headers: { Accept: 'application/json' },
    });
    const payload = await readCommandEnvelope(response);
    if (seedInputFeedback) seedInputFeedback.textContent = payload.message || 'Seed tracks removed from queue';
    await loadSeedPanel();
    refreshQueueFromProxy();
  } catch (err) {
    if (seedInputFeedback) seedInputFeedback.textContent = err.message;
  }
}

async function removeSeedPlaylist(playlistId) {
  try {
    const response = await fetch(`/api/seed/playlists/${encodeURIComponent(playlistId)}`, {
      method: 'DELETE',
      headers: { Accept: 'application/json' },
    });
    const payload = await readCommandEnvelope(response);
    if (seedInputFeedback) seedInputFeedback.textContent = payload.message || 'Playlist removed';
    await loadSeedPanel();
  } catch (err) {
    if (seedInputFeedback) seedInputFeedback.textContent = err.message;
  }
}

async function saveSeedSettings(clearQueueOnRequest) {
  try {
    await fetch('/api/seed/settings', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: JSON.stringify({ clearQueueOnRequest }),
    }).then(readCommandEnvelope);
  } catch (err) {
    if (seedInputFeedback) seedInputFeedback.textContent = err.message;
  }
}

if (addSeedPlaylistButton) {
  addSeedPlaylistButton.addEventListener('click', addSeedPlaylist);
}
if (seedPlaylistInput) {
  seedPlaylistInput.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      addSeedPlaylist();
    }
  });
}
if (clearQueueOnRequestCheckbox) {
  clearQueueOnRequestCheckbox.addEventListener('change', () => {
    saveSeedSettings(clearQueueOnRequestCheckbox.checked);
  });
}

if (twitchChatLog) {
  twitchChatLog.addEventListener('scroll', () => {
    twitchStickBottom = twitchChatLog.scrollHeight - twitchChatLog.scrollTop - twitchChatLog.clientHeight < 24;
  });
}

async function loadTwitchPanel() {
  await Promise.all([loadTwitchSettings(), loadTwitchLive()]);
}

async function loadTwitchSettings() {
  if (!twitchChannelInput) return;
  try {
    const response = await fetch('/api/config');
    const payload = await response.json();
    if (!response.ok || payload.exitCode !== 0) {
      throw new Error(payload.message || 'Unable to load Twitch settings');
    }
    twitchChannelInput.value = payload.data?.twitchChannel || '';
    twitchNickInput.value = payload.data?.twitchNick || '';
  } catch (err) {
    setTwitchFeedback(`Failed to load settings: ${err.message}`);
  }
}

async function loadTwitchLive() {
  const wantChat = activeAdminPage() === 'chat';
  try {
    const fetches = [fetch('/api/twitch')];
    if (wantChat) {
      fetches.push(fetch('/api/twitch/chat'));
    }
    const [statusRes, chatRes] = await Promise.all(fetches);
    const status = await statusRes.json();
    if (!statusRes.ok || status.exitCode !== 0) {
      throw new Error(status.message || 'Unable to load Twitch status');
    }
    renderTwitchStatus(status.data || {});
    if (wantChat && chatRes) {
      const chat = await chatRes.json();
      if (chatRes.ok && chat.exitCode === 0) {
        renderTwitchChat(chat.data?.messages || []);
      }
    }
  } catch (err) {
    if (twitchStatusLine) {
      twitchStatusLine.textContent = `Twitch: error (${err.message})`;
      twitchStatusLine.classList.remove('seed-status--active');
    }
  }
}

function renderTwitchStatus(data) {
  if (!twitchStatusLine) return;
  const channel = data.channel ? `#${data.channel}` : '(no channel)';
  const nick = data.nick || '(no nick)';
  let text = `Twitch: disconnected as ${nick} on ${channel}`;
  if (data.connected) {
    text = `Twitch: connected as ${nick} on ${channel}`;
  } else if (data.wanted) {
    text = `Twitch: connecting as ${nick} on ${channel}…`;
  }
  if (data.lastError) {
    text += ` (${data.lastError})`;
  }
  if (data.refreshSet) {
    text += ' · logged in';
  } else if (data.oauthSet) {
    text += ' · token saved';
  }
  twitchStatusLine.textContent = text;
  twitchStatusLine.classList.toggle('seed-status--active', !!data.connected);
}

function renderTwitchChat(messages) {
  if (!twitchChatLog) return;
  const key = messages.map((m) => m.id).join(',');
  if (key === twitchLastChatKey) return;
  twitchLastChatKey = key;
  if (!messages.length) {
    twitchChatLog.innerHTML = '<div class="muted">No chat yet.</div>';
    return;
  }
  twitchChatLog.innerHTML = messages.map((m) => {
    const color = typeof m.color === 'string' && /^#[0-9A-Fa-f]{6}$/.test(m.color)
      ? ` style="color: ${m.color}"`
      : '';
    const cmdClass = isChatCommandText(m.text) ? ' twitch-log__line--cmd' : '';
    return `<div class="twitch-log__line${cmdClass}"><span class="twitch-log__nick"${color}>${escapeHtml(m.nick)}</span>${escapeHtml(m.text)}</div>`;
  }).join('');
  if (twitchStickBottom) {
    twitchChatLog.scrollTop = twitchChatLog.scrollHeight;
  }
}

function setTwitchFeedback(text) {
  if (twitchFormFeedback) {
    twitchFormFeedback.textContent = text;
  }
}

async function saveTwitchSettings() {
  const body = {
    twitchChannel: (twitchChannelInput?.value || '').trim(),
  };
  const response = await fetch('/api/config', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  const payload = await response.json();
  if (!response.ok || payload.exitCode !== 0) {
    throw new Error(payload.message || 'Unable to save Twitch settings');
  }
  twitchChannelInput.value = payload.data?.twitchChannel || body.twitchChannel;
  if (twitchNickInput) {
    twitchNickInput.value = payload.data?.twitchNick || twitchNickInput.value;
  }
}

async function postTwitch(path) {
  const response = await fetch(path, { method: 'POST' });
  const payload = await response.json();
  if (!response.ok || payload.exitCode !== 0) {
    throw new Error(payload.message || `Request failed: ${path}`);
  }
  renderTwitchStatus(payload.data || {});
}

if (twitchSaveButton) {
  twitchSaveButton.addEventListener('click', async () => {
    try {
      await saveTwitchSettings();
      setTwitchFeedback('Saved.');
      await loadTwitchLive();
    } catch (err) {
      setTwitchFeedback(err.message);
    }
  });
}

if (twitchLoginButton) {
  twitchLoginButton.addEventListener('click', async () => {
    try {
      await saveTwitchSettings();
      const response = await fetch('/api/twitch/oauth/start', { method: 'POST' });
      const payload = await response.json();
      if (!response.ok || payload.exitCode !== 0) {
        throw new Error(payload.message || 'Unable to start Twitch login');
      }
      const url = payload.data?.url;
      if (!url) {
        throw new Error('Twitch login URL missing');
      }
      setTwitchFeedback('Approve as the bot account in the Twitch window.');
      const popup = window.open(url, '_blank', 'noopener');
      if (!popup) {
        window.location.href = url;
      }
    } catch (err) {
      setTwitchFeedback(err.message);
    }
  });
}

if (twitchConnectButton) {
  twitchConnectButton.addEventListener('click', async () => {
    try {
      await saveTwitchSettings();
      await postTwitch('/api/twitch/connect');
      setTwitchFeedback('Connecting…');
    } catch (err) {
      setTwitchFeedback(err.message);
    }
  });
}

if (twitchDisconnectButton) {
  twitchDisconnectButton.addEventListener('click', async () => {
    try {
      await postTwitch('/api/twitch/disconnect');
      setTwitchFeedback('Disconnected.');
    } catch (err) {
      setTwitchFeedback(err.message);
    }
  });
}

function setChatCmdsUi(enabled) {
  const on = Boolean(enabled);
  if (chatCmdsPill) {
    const label = chatCmdsPill.querySelector('.cmd-pill__label');
    if (label) {
      label.textContent = on ? 'Chat cmds ON' : 'Chat cmds OFF';
    }
    chatCmdsPill.classList.toggle('is-on', on);
    chatCmdsPill.classList.toggle('is-off', !on);
    chatCmdsPill.setAttribute('aria-pressed', on ? 'true' : 'false');
  }
  if (chatCmdsEnabled) {
    chatCmdsEnabled.checked = on;
  }
}

async function refreshChatCmdsPill() {
  try {
    const response = await fetch('/cmd/jb/commands');
    const payload = await response.json();
    if (!response.ok || payload.exitCode !== 0) return;
    setChatCmdsUi(!!payload.data?.enabled);
  } catch (_) {
    /* pill is best-effort */
  }
}

async function setChatCmdsEnabled(enabled) {
  const path = enabled ? '/cmd/jb/commands/on' : '/cmd/jb/commands/off';
  const response = await fetch(path);
  const payload = await response.json();
  if (!response.ok || payload.exitCode !== 0) {
    throw new Error(payload.message || 'Unable to set chat commands');
  }
  setChatCmdsUi(!!payload.data?.enabled);
}

function setCmdFeedback(text) {
  if (cmdFeedback) {
    cmdFeedback.textContent = text;
  }
}

function isChatCommandText(text) {
  return /^(?:!sr|!songrequest|!q|!queue|!si|!songinfo)(?:\s|$)/i.test(String(text || '').trim());
}

function safeHttpsUrl(value) {
  return typeof value === 'string' && /^https:\/\/[a-z0-9.-]+/i.test(value) ? value : '';
}

function renderCmdHistory(entries) {
  if (!cmdHistoryLog) return;
  if (!entries.length) {
    cmdHistoryLog.innerHTML = '<div class="muted">No commands yet.</div>';
    return;
  }
  cmdHistoryLog.innerHTML = entries.map((e) => {
    const nick = escapeHtml(e.nick || '');
    const cmd = escapeHtml(e.cmd || '');
    const args = e.args ? ` ${escapeHtml(e.args)}` : '';
    const reply = e.reply ? `<div class="cmd-history__reply">${escapeHtml(e.reply)}</div>` : '';
    const p = e.preview || null;
    let card = '';
    if (p && (p.title || p.artist || p.thumb)) {
      const title = escapeHtml(p.title || '');
      const artist = escapeHtml(p.artist || '');
      const meta = [title, artist].filter(Boolean).join(' — ');
      const thumb = safeHttpsUrl(p.thumb);
      const img = thumb
        ? `<img class="cmd-history__thumb" src="${escapeHtml(thumb)}" alt="">`
        : '';
      const inner = `${img}<span class="cmd-history__meta">${meta}</span>`;
      const url = safeHttpsUrl(p.url);
      card = url
        ? `<a class="cmd-history__card" href="${escapeHtml(url)}" target="_blank" rel="noopener noreferrer">${inner}</a>`
        : `<div class="cmd-history__card">${inner}</div>`;
    }
    return `<div class="cmd-history__row">
      <div class="cmd-history__head"><span class="cmd-history__nick">${nick}</span>!${cmd}${args}</div>
      ${card}
      ${reply}
    </div>`;
  }).join('');
}

function fillCmdCooldownRows(map) {
  if (!cmdCooldownRows) return;
  const src = map || {};
  cmdCooldownRows.innerHTML = CHAT_COMMAND_ROWS.map((row) => {
    const cd = src[row.id] || {};
    const gcd = Number.isFinite(Number(cd.gcd)) ? Number(cd.gcd) : 0;
    const ucd = Number.isFinite(Number(cd.ucd)) ? Number(cd.ucd) : 0;
    return `<tr>
      <td>${row.aliases}</td>
      <td><input type="number" min="0" step="1" data-cmd="${row.id}" data-field="gcd" value="${gcd}" aria-label="${row.id} GCD seconds" /></td>
      <td><input type="number" min="0" step="1" data-cmd="${row.id}" data-field="ucd" value="${ucd}" aria-label="${row.id} UCD seconds" /></td>
    </tr>`;
  }).join('');
}

async function loadCommandsPanel() {
  try {
    const [cfgRes, enRes, histRes] = await Promise.all([
      fetch('/api/config'),
      fetch('/cmd/jb/commands'),
      fetch('/api/twitch/history'),
    ]);
    const cfg = await cfgRes.json();
    const en = await enRes.json();
    if (!cfgRes.ok || cfg.exitCode !== 0) {
      throw new Error(cfg.message || 'Unable to load commands');
    }
    fillCmdCooldownRows(cfg.data?.twitchCommands);
    if (enRes.ok && en.exitCode === 0) {
      setChatCmdsUi(!!en.data?.enabled);
    } else {
      setChatCmdsUi(!!cfg.data?.twitchCommandsEnabled);
    }
    if (histRes.ok) {
      const hist = await histRes.json();
      if (hist.exitCode === 0) {
        renderCmdHistory(hist.data?.entries || []);
      }
    }
  } catch (err) {
    setCmdFeedback(err.message);
  }
}

if (chatCmdsPill) {
  chatCmdsPill.addEventListener('click', async () => {
    const next = !chatCmdsPill.classList.contains('is-on');
    try {
      await setChatCmdsEnabled(next);
    } catch (_) {
      /* keep previous pill state */
    }
  });
}

if (chatCmdsEnabled) {
  chatCmdsEnabled.addEventListener('change', async () => {
    try {
      await setChatCmdsEnabled(chatCmdsEnabled.checked);
      setCmdFeedback('');
    } catch (err) {
      chatCmdsEnabled.checked = !chatCmdsEnabled.checked;
      setCmdFeedback(err.message);
    }
  });
}

if (saveCmdCooldowns) {
  saveCmdCooldowns.addEventListener('click', async () => {
    const twitchCommands = {};
    cmdCooldownRows?.querySelectorAll('input[data-cmd]').forEach((input) => {
      const id = input.dataset.cmd;
      if (!twitchCommands[id]) {
        twitchCommands[id] = { gcd: 0, ucd: 0 };
      }
      const n = Number.parseInt(input.value, 10);
      twitchCommands[id][input.dataset.field] = Number.isFinite(n) ? Math.max(0, n) : 0;
    });
    try {
      const response = await fetch('/api/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ twitchCommands }),
      });
      const payload = await response.json();
      if (!response.ok || payload.exitCode !== 0) {
        throw new Error(payload.message || 'Unable to save cooldowns');
      }
      fillCmdCooldownRows(payload.data?.twitchCommands);
      setCmdFeedback('Saved.');
    } catch (err) {
      setCmdFeedback(err.message);
    }
  });
}

if (twitchChatForm) {
  twitchChatForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const message = (twitchChatInput?.value || '').trim();
    if (!message) return;
    try {
      const response = await fetch('/api/twitch/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ message }),
      });
      const payload = await response.json();
      if (!response.ok || payload.exitCode !== 0) {
        throw new Error(payload.message || 'Send failed');
      }
      twitchChatInput.value = '';
      if (chatFormFeedback) chatFormFeedback.textContent = '';
      setTwitchFeedback('');
      await loadTwitchLive();
    } catch (err) {
      if (chatFormFeedback) chatFormFeedback.textContent = err.message;
      setTwitchFeedback(err.message);
    }
  });
}

async function loadSettings() {
  try {
    const response = await fetch('/api/config');
    const payload = await response.json();
    if (!response.ok || payload.exitCode !== 0) {
      throw new Error(payload.message || 'Unable to load settings');
    }
    renderBlacklist(payload.data?.blacklist || []);
    if (Number.isFinite(payload.data?.maxDuration)) {
      maxDurationInput.value = String(payload.data.maxDuration);
    }
    settingsOutput.textContent = JSON.stringify(payload.data || {}, null, 2);
  } catch (err) {
    settingsOutput.textContent = `Failed to load settings: ${err.message}`;
  }
}

function getCurrentBlacklist() {
  return Array.from(blacklistList.querySelectorAll('li[data-entry]')).map((item) => item.dataset.entry || '');
}

function renderBlacklist(items) {
  blacklistList.innerHTML = '';
  if (!items.length) {
    const item = document.createElement('li');
    item.textContent = 'No blacklist entries yet';
    blacklistList.appendChild(item);
    return;
  }

  items.forEach((entry) => {
    const item = document.createElement('li');
    item.dataset.entry = entry;
    // Entries come from Import too; escape so a crafted entry cannot inject markup.
    item.innerHTML = `<span>${escapeHtml(entry)}</span><button type="button" aria-label="Remove ${escapeHtml(entry)}">${icon('x')}</button>`;
    blacklistList.appendChild(item);
  });

  blacklistList.querySelectorAll('button').forEach((button) => {
    button.addEventListener('click', () => removeBlacklistEntry(button.closest('li')?.dataset.entry || ''));
  });
}

async function saveSettings(blacklist, maxDuration = Number(maxDurationInput.value) || 600) {
  const response = await fetch('/api/config', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ blacklist: blacklist.filter(Boolean), maxDuration }),
  });
  const payload = await response.json();
  if (!response.ok || payload.exitCode !== 0) {
    throw new Error(payload.message || 'Unable to save settings');
  }
  settingsOutput.textContent = `Saved settings\n${JSON.stringify(payload.data || {}, null, 2)}`;
  renderBlacklist(payload.data?.blacklist || []);
}

async function addBlacklistEntry() {
  const value = blacklistInput.value.trim();
  if (!value) return;
  const current = getCurrentBlacklist();
  const next = [...new Set([...current, value])];
  blacklistInput.value = '';
  try {
    await saveSettings(next);
  } catch (err) {
    settingsOutput.textContent = `Failed to save settings: ${err.message}`;
  }
}

async function removeBlacklistEntry(value) {
  const current = getCurrentBlacklist();
  const next = current.filter((entry) => entry !== value);
  try {
    await saveSettings(next);
  } catch (err) {
    settingsOutput.textContent = `Failed to save settings: ${err.message}`;
  }
}

function setImportFeedback(text) {
  if (importFeedback) {
    importFeedback.textContent = text;
  }
}

async function importConfigFile() {
  const file = importFileInput?.files?.[0];
  if (!file) {
    setImportFeedback('Pick a jukeboks.json or jb2.json file first.');
    return;
  }
  setImportFeedback('Importing…');
  try {
    const text = await file.text();
    const parsed = JSON.parse(text);
    const response = await fetch('/api/config/import', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: JSON.stringify(parsed),
    });
    const payload = await readCommandEnvelope(response);
    const source = payload.data?.source || 'config';
    setImportFeedback(payload.message || `Imported ${source}`);
    loadSettings();
  } catch (err) {
    setImportFeedback(err.message || 'Import failed');
  }
}

if (importConfigButton) {
  importConfigButton.addEventListener('click', importConfigFile);
}

async function loadAboutPanel() {
  if (!aboutVersion) return;
  try {
    const response = await fetch('/health');
    const payload = await response.json();
    const ver = payload.data?.version;
    aboutVersion.textContent = ver ? `version ${ver}` : 'version unknown';
  } catch (err) {
    aboutVersion.textContent = `version unavailable (${err.message})`;
  }
}

addBlacklistButton.addEventListener('click', addBlacklistEntry);
blacklistInput.addEventListener('keydown', (event) => {
  if (event.key === 'Enter') {
    event.preventDefault();
    addBlacklistEntry();
  }
});

saveMaxDurationButton.addEventListener('click', async () => {
  try {
    const blacklist = getCurrentBlacklist();
    await saveSettings(blacklist, Number(maxDurationInput.value) || 600);
  } catch (err) {
    settingsOutput.textContent = `Failed to save settings: ${err.message}`;
  }
});

runButton.addEventListener('click', runCommand);
copyButton.addEventListener('click', async () => {
  try {
    await copyText(urlInput.value, urlInput);
    copyButton.innerHTML = icon('check');
    copyButton.setAttribute('aria-label', 'Copied');
    window.setTimeout(() => {
      copyButton.innerHTML = icon('copy');
      copyButton.setAttribute('aria-label', 'Copy');
    }, 1200);
  } catch (err) {
    copyButton.innerHTML = icon('alert-circle');
    copyButton.setAttribute('aria-label', 'Copy failed');
  }
});
let playbackState = {
  isPaused: true,
  position: 0,
  duration: 0,
  title: '',
  artist: '',
  videoId: '',
  hasSong: false,
  shuffle: false,
  queue: [],
  queueStatus: 'empty', // ok | empty | error | unavailable
};
let queueRefreshToken = 0;
let nowPlayingRefreshToken = 0;
let previousQueueExpanded = false;
let previousQueueRendered = false;
let queueActionFeedback = null;
let queueActionFeedbackTimer = null;
let queueDragActive = false;
let queueDragFromIndex = null;
let queueMoveInFlight = false;
let queueRefreshDebounceTimer = null;
let progressStallTimer = null;

// YTMD position ticks are often ~1s; keep headroom so we don't false-pause between ticks.
const PROGRESS_STALL_MS = 1800;

function isNowPlayingSocketOpen() {
  return window.__jukeboksSocket?.readyState === WebSocket.OPEN;
}

function scheduleQueueRefreshFromWS(delayMs = 800) {
  if (document.hidden || activeAdminPage() !== 'queue') return;
  window.clearTimeout(queueRefreshDebounceTimer);
  queueRefreshDebounceTimer = window.setTimeout(() => {
    refreshQueueFromProxy();
  }, delayMs);
}

function icon(name) {
  return `<svg class="icon" aria-hidden="true"><use href="/admin/icons.svg#${name}"></use></svg>`;
}

// Copy in the click itself: clipboard.writeText() is async and loses the
// user gesture, and it throws if the document is not focused (OBS dock).
async function copyText(text, input) {
  if (input) {
    try {
      input.focus();
      input.select();
      if (document.execCommand('copy')) return;
    } catch (_) {
      /* fall through */
    }
  }
  window.focus();
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  throw new Error('copy failed');
}

function setPlayGlyph(paused) {
  if (!playState) return;
  playState.innerHTML = icon(paused ? 'player-pause' : 'player-play');
  playState.setAttribute('aria-label', paused ? 'Paused' : 'Playing');
}

function escapeHtml(value) {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function refreshPlaybackFromProxyIfNeeded() {
  if (!isNowPlayingSocketOpen()) {
    refreshNowPlayingFromProxy();
    refreshShuffleFromProxy();
  }
}

function stopAdminEvents() {
  if (adminEvents) {
    adminEvents.close();
    adminEvents = null;
  }
}

function snapshotAdminLive() {
  refreshPlaybackFromProxyIfNeeded();
  if (activeAdminPage() === 'queue') {
    refreshQueueFromProxy();
  } else if (activeAdminPage() === 'seed') {
    loadSeedPanel();
  } else if (activeAdminPage() === 'twitch') {
    loadTwitchLive();
  } else if (activeAdminPage() === 'chat') {
    loadTwitchLive();
  } else if (activeAdminPage() === 'commands') {
    loadCommandsPanel();
  }
  refreshChatCmdsPill();
}

function onAdminEvent(kind, handler) {
  if (!adminEvents) return;
  adminEvents.addEventListener(kind, (event) => {
    try {
      handler(JSON.parse(event.data || '{}'));
    } catch (err) {
      console.error(`Failed to parse ${kind} event`, err);
    }
  });
}

function startAdminEvents() {
  stopAdminEvents();
  if (document.hidden) return;
  adminEvents = new EventSource('/api/events');
  adminEvents.onopen = () => {
    snapshotAdminLive();
  };
  onAdminEvent('twitch', (data) => {
    renderTwitchStatus(data);
  });
  onAdminEvent('chat', (data) => {
    if (activeAdminPage() !== 'chat') return;
    renderTwitchChat(data.messages || []);
  });
  onAdminEvent('seed', () => {
    if (activeAdminPage() === 'seed') loadSeedPanel();
  });
  onAdminEvent('commands', (data) => {
    if (typeof data.enabled === 'boolean') {
      setChatCmdsUi(data.enabled);
    } else {
      refreshChatCmdsPill();
    }
    if (activeAdminPage() === 'commands') loadCommandsPanel();
  });
  onAdminEvent('cmdHistory', (data) => {
    if (activeAdminPage() !== 'commands') return;
    renderCmdHistory(data.entries || []);
  });
  onAdminEvent('queue', () => {
    scheduleQueueRefreshFromWS();
  });
}

function pauseAdminLive() {
  stopAdminEvents();
}

function resumeAdminLive() {
  snapshotAdminLive();
  connectNowPlaying();
  startAdminEvents();
  updateCurrentMetaMarquee();
}

function setAdminPaused(isPaused) {
  const next = Boolean(isPaused);
  if (playbackState.isPaused === next) {
    if (playbackState.hasSong) {
      setPlayGlyph(next);
    }
    updatePlaybackButtons();
    return;
  }
  playbackState.isPaused = next;
  if (playbackState.hasSong) {
    setPlayGlyph(next);
  }
  updatePlaybackButtons();
}

// YTMD keeps sending POSITION_CHANGED while playing and stops when paused.
function notePlaybackProgress() {
  window.clearTimeout(progressStallTimer);
  progressStallTimer = window.setTimeout(() => {
    if (!playbackState.hasSong || playbackState.isPaused) return;
    setAdminPaused(true);
  }, PROGRESS_STALL_MS);
}

function applyPositionFromWS(position) {
  const next = Math.max(0, Number(position) || 0);
  const advanced = next > playbackState.position + 0.05;
  playbackState.position = next;
  if (advanced && playbackState.hasSong) {
    setAdminPaused(false);
    notePlaybackProgress();
  } else if (playbackState.hasSong) {
    setPlayGlyph(playbackState.isPaused);
  }
  currentTime.textContent = formatTime(playbackState.position);
  updateProgressBar();
}

function resolveIsPaused(...candidates) {
  for (const obj of candidates) {
    if (obj && typeof obj.isPlaying === 'boolean') return !obj.isPlaying;
  }
  for (const obj of candidates) {
    if (obj && typeof obj.isPaused === 'boolean') return obj.isPaused;
  }
  return null;
}

function applyPlayerStateFromWS(data) {
  const isPaused = resolveIsPaused(data);
  if (isPaused === null) return;
  setAdminPaused(isPaused);
  if (!isPaused) {
    notePlaybackProgress();
  } else {
    window.clearTimeout(progressStallTimer);
  }
  if (data.position != null) {
    playbackState.position = Math.max(0, Number(data.position) || 0);
    currentTime.textContent = formatTime(playbackState.position);
    updateProgressBar();
  }
}

async function readCommandEnvelope(response) {
  let payload = null;
  try {
    payload = await response.json();
  } catch (_) {
    payload = null;
  }

  if (!response.ok) {
    if (response.status === 404 && response.url.includes('/api/seed')) {
      throw new Error('Seed API not found — restart jukeboks (just run) to load the latest server');
    }
    const message = payload?.message || `status ${response.status}`;
    throw new Error(message);
  }

  if (payload && typeof payload.exitCode === 'number' && payload.exitCode !== 0) {
    throw new Error(payload.message || 'Command failed');
  }

  return payload;
}

function setPreviousQueueExpanded(expanded, button = null, sublist = null) {
  previousQueueExpanded = Boolean(expanded);
  if (button) {
    const label = previousQueueExpanded ? 'Hide previous' : 'Previous';
    button.innerHTML = `${icon(previousQueueExpanded ? 'chevron-up' : 'chevron-right')}<span>${label}</span>`;
    button.setAttribute('aria-expanded', previousQueueExpanded ? 'true' : 'false');
  }
  if (sublist) {
    sublist.classList.toggle('is-collapsed', !previousQueueExpanded);
    sublist.hidden = !previousQueueExpanded;
    sublist.setAttribute('data-expanded', previousQueueExpanded ? 'true' : 'false');
  }
}

function syncPreviousToggle(hasPrevious) {
  if (!previousToggle) return;
  previousToggle.hidden = !hasPrevious;
  if (!hasPrevious) return;
  setPreviousQueueExpanded(previousQueueExpanded, previousToggle, null);
}

function handleQueueToggle(event) {
  const button = event.target.closest('[data-toggle="previous"]');
  if (!button) return;
  const sublist = queueList?.querySelector('[data-previous-list]');
  if (!sublist) return;
  event.preventDefault();
  setPreviousQueueExpanded(sublist.hidden, button, sublist);
}

function shouldHandleQueueAction(event) {
  return Boolean(event.target.closest('[data-queue-action]'));
}

function setQueueActionFeedback(message, kind = 'success') {
  queueActionFeedback = { message: String(message || ''), kind };
  renderQueue();

  if (queueActionFeedbackTimer) {
    window.clearTimeout(queueActionFeedbackTimer);
  }
  const ttl = kind === 'error' ? 4000 : 1400;
  queueActionFeedbackTimer = window.setTimeout(() => {
    queueActionFeedback = null;
    renderQueue();
  }, ttl);
}

function resetQueueActionButtonState(button, action) {
  if (!button) return;
  button.classList.remove('is-pending', 'is-success', 'is-error');
  button.disabled = false;
  button.innerHTML = icon(action === 'delete' ? 'x' : 'player-play');
}

async function handleQueueItemAction(event) {
  const button = event.target.closest('[data-queue-action]');
  if (!button) return;

  const action = button.dataset.queueAction;
  const queueIndex = Number(button.dataset.queueIndex);
  if (!Number.isFinite(queueIndex)) return;

  event.preventDefault();

  button.classList.add('is-pending');
  button.disabled = true;

  const path = action === 'delete'
    ? `/cmd/ytmd/queue/${queueIndex}/delete`
    : '/cmd/ytmd/queue/patch';
  const method = action === 'delete' ? 'DELETE' : 'PATCH';
  const init = {
    method,
    headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
  };

  if (action !== 'delete') {
    init.body = JSON.stringify({ index: queueIndex });
  }

  try {
    await readCommandEnvelope(await fetch(path, init));

    button.classList.remove('is-pending');
    button.classList.add('is-success');
    button.innerHTML = icon('check');
    setQueueActionFeedback(action === 'delete' ? 'Deleted queue item' : 'Started queue item', 'success');

    window.setTimeout(() => {
      refreshQueueFromProxy();
      refreshNowPlayingFromProxy();
    }, 700);
  } catch (err) {
    button.classList.remove('is-pending');
    button.classList.add('is-error');
    button.innerHTML = icon('alert-circle');
    setQueueActionFeedback(err?.message || (action === 'delete' ? 'Unable to delete queue item' : 'Unable to start queue item'), 'error');
    console.error('Queue action failed', err);
  } finally {
    window.setTimeout(() => {
      resetQueueActionButtonState(button, action);
    }, 900);
  }
}

async function handleClearQueue() {
  if (!clearQueueButton) return;
  if (!window.confirm('Clear the entire YTMD queue?')) {
    return;
  }

  clearQueueButton.disabled = true;
  try {
    await readCommandEnvelope(await fetch('/cmd/ytmd/queue/delete', {
      method: 'DELETE',
      headers: { Accept: 'application/json' },
    }));
    setQueueActionFeedback('Queue cleared', 'success');
    playbackState.queue = [];
    playbackState.queueStatus = 'empty';
    renderQueue();
    window.setTimeout(() => {
      refreshQueueFromProxy();
      refreshNowPlayingFromProxy();
    }, 500);
  } catch (err) {
    setQueueActionFeedback(err?.message || 'Unable to clear queue', 'error');
    console.error('Clear queue failed', err);
  } finally {
    clearQueueButton.disabled = false;
  }
}

async function moveQueueItem(fromIndex, toIndex) {
  if (!Number.isFinite(fromIndex) || !Number.isFinite(toIndex) || fromIndex === toIndex) {
    return;
  }
  if (queueMoveInFlight) {
    return;
  }

  queueMoveInFlight = true;
  if (queueList) {
    queueList.classList.add('is-reordering');
  }

  try {
    await readCommandEnvelope(await fetch(`/cmd/ytmd/queue/${fromIndex}/patch`, {
      method: 'PATCH',
      headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
      body: JSON.stringify({ toIndex }),
    }));
    setQueueActionFeedback('Moved queue item', 'success');
    window.setTimeout(() => {
      refreshQueueFromProxy();
      refreshNowPlayingFromProxy();
    }, 700);
  } catch (err) {
    setQueueActionFeedback(err?.message || 'Unable to move queue item', 'error');
    console.error('Queue move failed', err);
    refreshQueueFromProxy();
  } finally {
    queueMoveInFlight = false;
    if (queueList) {
      queueList.classList.remove('is-reordering');
    }
  }
}

function clearQueueDropTargets() {
  if (!queueList) return;
  queueList.querySelectorAll('.queue-item.is-drop-target').forEach((el) => {
    el.classList.remove('is-drop-target');
  });
}

function clearQueueDragState() {
  queueDragActive = false;
  queueDragFromIndex = null;
  document.body.classList.remove('is-queue-dragging');
  if (!queueList) return;
  queueList.querySelectorAll('.queue-item.is-dragging').forEach((el) => {
    el.classList.remove('is-dragging');
  });
  clearQueueDropTargets();
}

function setQueueDropTarget(item) {
  clearQueueDropTargets();
  if (!item) return;
  const toIndex = Number(item.dataset.queueIndex);
  if (Number.isFinite(toIndex) && toIndex !== queueDragFromIndex) {
    item.classList.add('is-drop-target');
  }
}

function bindQueueDragDrop() {
  if (!queueList || queueList.dataset.dndBound === '1') return;
  queueList.dataset.dndBound = '1';

  queueList.addEventListener('dragstart', (event) => {
    const handle = event.target.closest('.queue-drag-handle');
    const item = event.target.closest('.queue-item[data-queue-draggable="true"]');
    if (!handle || !item || !queueList.contains(item)) {
      event.preventDefault();
      return;
    }
    const fromIndex = Number(item.dataset.queueIndex);
    if (!Number.isFinite(fromIndex)) {
      event.preventDefault();
      return;
    }
    queueDragActive = true;
    queueDragFromIndex = fromIndex;
    item.classList.add('is-dragging');
    document.body.classList.add('is-queue-dragging');
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('text/plain', String(fromIndex));
  });

  queueList.addEventListener('dragover', (event) => {
    const target = event.target.closest('.queue-item[data-queue-draggable="true"]');
    if (!target || !queueList.contains(target)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
    setQueueDropTarget(target);
  });

  queueList.addEventListener('drop', async (event) => {
    const target = event.target.closest('.queue-item[data-queue-draggable="true"]');
    if (!target || !queueList.contains(target)) return;
    event.preventDefault();
    const fromIndex = queueDragFromIndex;
    const toIndex = Number(target.dataset.queueIndex);
    clearQueueDragState();
    if (Number.isFinite(fromIndex) && Number.isFinite(toIndex) && fromIndex !== toIndex) {
      await moveQueueItem(fromIndex, toIndex);
    } else {
      scheduleQueueRefreshFromWS(0);
    }
  });

  queueList.addEventListener('dragend', () => {
    const wasDragging = queueDragActive;
    clearQueueDragState();
    if (wasDragging && !queueMoveInFlight) {
      scheduleQueueRefreshFromWS(0);
    }
  });
}

if (queueList) {
  queueList.addEventListener('click', (event) => {
    if (shouldHandleQueueAction(event)) {
      handleQueueItemAction(event);
    }
  });
  bindQueueDragDrop();
}

if (previousToggle) {
  previousToggle.addEventListener('click', handleQueueToggle);
}

if (clearQueueButton) {
  clearQueueButton.addEventListener('click', () => {
    handleClearQueue();
  });
}

function formatTime(seconds) {
  const totalSeconds = Number.isFinite(seconds) ? Math.max(0, Math.floor(seconds)) : 0;
  const mins = String(Math.floor(totalSeconds / 60)).padStart(2, '0');
  const secs = String(totalSeconds % 60).padStart(2, '0');
  return `${mins}:${secs}`;
}

function getCurrentMetaTrack() {
  if (!currentMeta) return null;

  let track = currentMeta.querySelector('.now-playing__headline-track');
  if (track) {
    return track;
  }

  track = document.createElement('span');
  track.className = 'now-playing__headline-track';
  track.textContent = currentMeta.textContent || '';
  currentMeta.textContent = '';
  currentMeta.appendChild(track);
  return track;
}

function setCurrentMetaText(value) {
  const track = getCurrentMetaTrack();
  if (!track) return;
  track.textContent = value;
}

function getCurrentMetaText() {
  const track = getCurrentMetaTrack();
  return track ? track.textContent || '' : '';
}

function updateCurrentMetaMarquee() {
  if (!currentMeta || !currentHeadline) return;
  const currentMetaTrack = getCurrentMetaTrack();
  if (!currentMetaTrack) return;

  currentMeta.classList.remove('is-marquee');
  currentMeta.style.removeProperty('--marquee-distance');

  window.requestAnimationFrame(() => {
    const marqueeEndPadding = 24;
    const availableWidth = currentHeadline.clientWidth;
    const textWidth = currentMetaTrack.scrollWidth;
    const overflowWidth = textWidth - availableWidth;

    if (overflowWidth > 4) {
      currentMeta.style.setProperty('--marquee-distance', `${overflowWidth + marqueeEndPadding}px`);
      currentMeta.classList.add('is-marquee');
    }
  });
}

function setToggleButtonState(button, isOn) {
  if (!button) return;
  button.classList.toggle('is-active', Boolean(isOn));
  button.setAttribute('aria-pressed', isOn ? 'true' : 'false');
}

function updatePlaybackButtons() {
  const isPlayingNow = playbackState.hasSong && !playbackState.isPaused;
  if (playerButtons.play && playerButtons.pause) {
    // Highlight the action available: Pause while playing, Play while paused.
    playerButtons.play.classList.toggle('is-active', playbackState.hasSong && playbackState.isPaused);
    playerButtons.pause.classList.toggle('is-active', isPlayingNow);
  }
  if (compactPlay) {
    compactPlay.innerHTML = icon(isPlayingNow ? 'player-pause' : 'player-play');
    compactPlay.setAttribute('aria-label', isPlayingNow ? 'Pause' : 'Play');
  }
  document.body.classList.toggle('is-playing', isPlayingNow);
  setToggleButtonState(playerButtons.shuffle, playbackState.shuffle);
}

function updateProgressBar() {
  const maxDuration = Math.max(1, playbackState.duration || 100);
  const position = Math.min(maxDuration, playbackState.position || 0);
  progressBar.max = String(maxDuration);
  progressBar.value = String(position);
  // Range value cannot paint a partial fill. The track gradient reads --seek.
  progressBar.style.setProperty('--seek', `${(position / maxDuration) * 100}%`);
  currentTime.textContent = formatTime(playbackState.position);
  totalTime.textContent = formatTime(playbackState.duration);
}

function setQueueCount(count) {
  if (!queueCount) return;
  queueCount.textContent = `(${count})`;
}


function renderQueue() {
  if (!queueList) return;
  if (queueDragActive || queueMoveInFlight) return;

  const feedbackMarkup = queueActionFeedback
    ? `<div class="queue-feedback queue-feedback--${escapeHtml(queueActionFeedback.kind)}">${escapeHtml(queueActionFeedback.message)}</div>`
    : '';

  if (playbackState.queueStatus === 'unavailable') {
    syncPreviousToggle(false);
    setQueueCount(0);
    queueList.innerHTML = `${feedbackMarkup}<div class="queue-empty-state">YTMD queue unavailable. Check that the player is reachable.</div>`;
    return;
  }

  if (playbackState.queueStatus === 'error') {
    syncPreviousToggle(false);
    setQueueCount(0);
    queueList.innerHTML = `${feedbackMarkup}<div class="queue-empty-state">Unable to load queue. Retrying…</div>`;
    return;
  }

  if (!playbackState.queue?.length || playbackState.queueStatus === 'empty') {
    syncPreviousToggle(false);
    setQueueCount(0);
    queueList.innerHTML = `${feedbackMarkup}<div class="queue-empty-state">Queue is empty.</div>`;
    return;
  }

  const visibleQueueItems = playbackState.queue.filter((item) => item && (item.title || item.artist || item.videoId));
  const previousItems = visibleQueueItems.filter((item) => item.kind === 'previous');
  const currentItem = visibleQueueItems.find((item) => item.kind === 'current');
  const nextItems = visibleQueueItems.filter((item) => item.kind === 'next');
  const previousExpanded = previousItems.length > 0 ? previousQueueExpanded : false;
  if (previousItems.length === 0) {
    previousQueueExpanded = false;
  }

  const buildItem = (item, kind) => {
    const displayText = escapeHtml([item.artist, item.title].filter(Boolean).join(' - '));
    const classes = [`queue-item`, kind === 'current' ? 'is-current' : '', kind === 'previous' ? 'is-previous' : ''].filter(Boolean).join(' ');
    const queueIndex = Number.isFinite(item?.queueIndex) ? item.queueIndex : '';
    const actionsDisabled = !Number.isFinite(item?.queueIndex);
    const canDrag = kind === 'next' && Number.isFinite(item?.queueIndex);
    const dragAttrs = canDrag
      ? ` data-queue-draggable="true" data-queue-index="${queueIndex}"`
      : '';
    const dragHandle = canDrag
      ? `<span class="queue-drag-handle" draggable="true" title="Drag to reorder" aria-label="Drag to reorder">${icon('grip-vertical')}</span>`
      : '';
    return `
      <div class="${classes}"${dragAttrs}>
        <div class="queue-row">
          ${dragHandle}
          <div class="queue-meta">${displayText || 'Untitled'}</div>
          <div class="queue-actions">
            <button class="queue-action-btn queue-action-btn--play" type="button" data-queue-action="play" data-queue-index="${queueIndex}" aria-label="Play from queue"${actionsDisabled ? ' disabled' : ''}>${icon('player-play')}</button>
            <button class="queue-action-btn queue-action-btn--delete" type="button" data-queue-action="delete" data-queue-index="${queueIndex}" aria-label="Delete from queue"${actionsDisabled ? ' disabled' : ''}>${icon('x')}</button>
          </div>
        </div>
      </div>
    `;
  };

  // Rows on screen: current plus upcoming. Previous stays behind its toggle.
  setQueueCount((currentItem ? 1 : 0) + nextItems.length);
  syncPreviousToggle(previousItems.length > 0);
  const previousMarkup = previousItems.length > 0
    ? `
        <div class="queue-sublist${previousExpanded ? '' : ' is-collapsed'}" data-previous-list ${previousExpanded ? '' : 'hidden'}>
          ${previousItems.map((entry) => buildItem(entry, 'previous')).join('')}
        </div>
    `
    : '';
  previousQueueRendered = previousItems.length > 0;

  const currentMarkup = currentItem
    ? buildItem(currentItem, 'current')
    : '<div class="queue-item"><div class="queue-meta">No current song highlighted.</div></div>';
  const nextMarkup = nextItems.length > 0
    ? nextItems.map((entry) => buildItem(entry, 'next')).join('')
    : '<div class="queue-item"><div class="queue-meta">No upcoming songs.</div></div>';

  queueList.innerHTML = `${feedbackMarkup}${previousMarkup}${currentMarkup}${nextMarkup}`;
}

async function refreshQueueFromProxy() {
  if (queueDragActive || queueMoveInFlight) {
    scheduleQueueRefreshFromWS(400);
    return;
  }

  const requestId = ++queueRefreshToken;
  try {
    const response = await fetch('/api/queue', { headers: { Accept: 'application/json' } });
    const payload = await response.json();
    if (requestId !== queueRefreshToken || queueDragActive || queueMoveInFlight) {
      if (queueDragActive || queueMoveInFlight) {
        scheduleQueueRefreshFromWS(400);
      }
      return;
    }

    const status = payload?.data?.status || 'unavailable';
    const items = Array.isArray(payload?.data?.items) ? payload.data.items : [];

    if (status === 'unavailable') {
      // Keep last good queue on transient YTMD blips; retry once.
      if (playbackState.queue?.length && playbackState.queueStatus === 'ok') {
        scheduleQueueRefreshFromWS(1200);
        return;
      }
      playbackState.queue = [];
      playbackState.queueStatus = 'unavailable';
      renderQueue();
      return;
    }

    playbackState.queue = items;
    playbackState.queueStatus = status === 'ok' && items.length === 0 ? 'empty' : status;
    renderQueue();
  } catch (err) {
    if (requestId !== queueRefreshToken || queueDragActive || queueMoveInFlight) {
      if (queueDragActive || queueMoveInFlight) {
        scheduleQueueRefreshFromWS(400);
      }
      return;
    }
    if (playbackState.queue?.length && playbackState.queueStatus === 'ok') {
      scheduleQueueRefreshFromWS(1200);
      return;
    }
    playbackState.queue = [];
    playbackState.queueStatus = 'unavailable';
    renderQueue();
  }
}

function applyNowPlaying(payload) {
  const source = payload?.song ?? payload?.data?.song ?? payload?.data ?? payload;
  const song = source?.song ?? source;
  // Do not take play/pause from PLAYER_INFO / GET /song — isPaused is stale and
  // PLAYER_INFO.isPlaying is derived from it. Use PLAYER_STATE_CHANGED + position ticks.
  const position = Number(
    payload?.position
      ?? source?.position
      ?? song?.elapsedSeconds
      ?? song?.position
      ?? song?.currentTime
      ?? song?.progress
      ?? 0,
  );
  const duration = Number(source?.songDuration ?? song?.songDuration ?? song?.duration ?? song?.length ?? 0);
  const nextTitle = song?.title || song?.name || song?.track || source?.title || source?.name || '';
  const nextArtist = song?.artist || song?.artistName || song?.channel || song?.author || source?.artist || source?.artistName || '';
  const nextVideoId = song?.videoId || song?.id || source?.videoId || source?.id || source?.video?.id || '';

  if (typeof payload?.shuffle === 'boolean') {
    playbackState.shuffle = payload.shuffle;
  } else if (typeof source?.shuffle === 'boolean') {
    playbackState.shuffle = source.shuffle;
  }

  if (nextTitle || nextArtist || duration || position) {
    playbackState.hasSong = true;
    playbackState.position = position || playbackState.position;
    playbackState.duration = duration || playbackState.duration;
    playbackState.title = nextTitle || playbackState.title;
    playbackState.artist = nextArtist || playbackState.artist;
    playbackState.videoId = nextVideoId || playbackState.videoId;
  }

  const metaText = `${playbackState.artist || 'Unknown artist'} • ${playbackState.title || 'Unknown title'}`;
  if (getCurrentMetaText() !== metaText) {
    setCurrentMetaText(metaText);
    updateCurrentMetaMarquee();
  }
  setPlayGlyph(playbackState.isPaused);
  currentTime.textContent = formatTime(playbackState.position);
  totalTime.textContent = formatTime(playbackState.duration);
  updatePlaybackButtons();
  updateProgressBar();
}

async function refreshNowPlayingFromProxy() {
  const requestId = ++nowPlayingRefreshToken;
  try {
    const response = await fetch('/cmd/ytmd/song/get', { headers: { Accept: 'application/json' } });
    if (!response.ok) {
      throw new Error(`status ${response.status}`);
    }

    const payload = await response.json();
    if (requestId !== nowPlayingRefreshToken) {
      return;
    }

    applyNowPlaying(payload);
  } catch (err) {
    if (requestId !== nowPlayingRefreshToken) {
      return;
    }

    setCurrentMetaText('YTMD unavailable');
    updateCurrentMetaMarquee();
    playbackState.isPaused = true;
    setPlayGlyph(true);
    currentTime.textContent = '00:00';
    totalTime.textContent = '00:00';
    updatePlaybackButtons();
  }
}

function connectNowPlaying() {
  const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
  const host = window.location.hostname || 'localhost';
  const port = new URLSearchParams(window.location.search).get('ytmdPort') || '26538';
  const wsUrl = `${protocol}://${host}:${port}/api/v1/ws`;

  if (window.__jukeboksSocket && window.__jukeboksSocket.readyState === WebSocket.OPEN) {
    return;
  }

  const socket = new WebSocket(wsUrl);
  socket.onopen = () => {
    setCurrentMetaText('Listening for YTMD…');
    currentMeta.classList.remove('is-marquee');
  };
  socket.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      if (data?.type === 'SHUFFLE_CHANGED' && typeof data.shuffle === 'boolean') {
        playbackState.shuffle = data.shuffle;
        updatePlaybackButtons();
        return;
      }
      if (data?.type === 'POSITION_CHANGED') {
        applyPositionFromWS(data.position);
        return;
      }
      if (data?.type === 'PLAYER_STATE_CHANGED') {
        applyPlayerStateFromWS(data);
        return;
      }
      if (data?.type === 'PLAYER_INFO' || data?.type === 'VIDEO_CHANGED') {
        applyNowPlaying(data);
        scheduleQueueRefreshFromWS();
        return;
      }
      if (data?.song || data?.artist || data?.title) {
        applyNowPlaying(data);
        scheduleQueueRefreshFromWS();
        return;
      }
      if (data?.position != null && !data?.song && !data?.title && !data?.artist) {
        applyPositionFromWS(data.position);
        return;
      }
      if (typeof data?.isPaused === 'boolean') {
        applyPlayerStateFromWS(data);
      }
    } catch (err) {
      console.error('Failed to parse YTMD socket payload', err);
    }
  };
  socket.onerror = () => {
    setCurrentMetaText('YTMD socket unavailable');
  };
  socket.onclose = () => {
    window.setTimeout(connectNowPlaying, 2000);
  };
  window.__jukeboksSocket = socket;
}

async function sendPlayerCommand(path) {
  try {
    await fetch(path, { method: 'POST', headers: { Accept: 'application/json' } });
    window.setTimeout(() => {
      // WS already owns play/pause; HTTP song.isPaused is stale and was wiping the glow.
      if (!isNowPlayingSocketOpen()) {
        refreshNowPlayingFromProxy();
      }
      refreshQueueFromProxy();
      refreshShuffleFromProxy();
    }, 600);
  } catch (err) {
    output.textContent = `Control request failed: ${err.message}`;
  }
}

async function submitSongRequest(inputOverride, playNow) {
  if (!songRequestInput) return;
  const input = String(inputOverride || songRequestInput.value).trim();
  if (!input) return;
  const params = new URLSearchParams({ input });
  if (playNow) params.set('now', '1');
  try {
    const response = await fetch(`/cmd/jb/songrequest?${params}`, {
      headers: { Accept: 'application/json' },
    });
    const payload = await readCommandEnvelope(response);
    songRequestInput.value = '';
    hideSongSearchPopup();
    setQueueActionFeedback(payload.message || (playNow ? 'Playing now' : 'Added to queue'), 'success');
    refreshQueueFromProxy();
    if (playNow) {
      refreshNowPlayingFromProxy();
    }
  } catch (err) {
    setQueueActionFeedback(err.message || 'Song request failed', 'error');
  }
}

let songSearchTimer = null;
let songSearchToken = 0;
let songSearchItems = [];
let songSearchHighlight = -1;

function songQueryLooksLikeUrl(value) {
  const v = String(value || '').trim();
  return /:\/\//.test(v) || /youtu\.be/i.test(v) || /spotify:/i.test(v);
}

function hideSongSearchPopup() {
  songSearchItems = [];
  songSearchHighlight = -1;
  if (!songSearchPopup) return;
  songSearchPopup.innerHTML = '';
  songSearchPopup.hidden = true;
}

function setSongSearchHighlight(index) {
  if (!songSearchPopup) return;
  const buttons = songSearchPopup.querySelectorAll('.search-popup__item');
  songSearchHighlight = index;
  buttons.forEach((btn, i) => {
    btn.classList.toggle('is-active', i === index);
  });
}

function renderSongSearchPopup(items) {
  if (!songSearchPopup) return;
  songSearchItems = items;
  songSearchHighlight = -1;
  if (!items.length) {
    hideSongSearchPopup();
    return;
  }
  songSearchPopup.innerHTML = items.map((item, i) => {
    const title = escapeHtml(item.title || item.videoId || 'Untitled');
    const artist = item.artist ? `<span class="search-popup__artist">${escapeHtml(item.artist)}</span>` : '';
    return `<li role="presentation"><button type="button" class="search-popup__item" role="option" data-search-index="${i}"><span class="search-popup__title">${title}</span>${artist}</button></li>`;
  }).join('');
  songSearchPopup.hidden = false;
}

async function fetchSongSearch(query) {
  const token = ++songSearchToken;
  try {
    const response = await fetch(`/cmd/jb/search?input=${encodeURIComponent(query)}`, {
      headers: { Accept: 'application/json' },
    });
    const payload = await readCommandEnvelope(response);
    if (token !== songSearchToken) return;
    const items = payload?.data?.items;
    renderSongSearchPopup(Array.isArray(items) ? items : []);
  } catch (_) {
    if (token !== songSearchToken) return;
    hideSongSearchPopup();
  }
}

function scheduleSongSearch() {
  window.clearTimeout(songSearchTimer);
  const query = songRequestInput?.value.trim() || '';
  if (query.length < 2 || songQueryLooksLikeUrl(query)) {
    hideSongSearchPopup();
    return;
  }
  songSearchTimer = window.setTimeout(() => {
    fetchSongSearch(query);
  }, 250);
}

function songRequestInputFromHighlight() {
  if (songSearchHighlight >= 0 && songSearchItems[songSearchHighlight]?.videoId) {
    return songSearchItems[songSearchHighlight].videoId;
  }
  return '';
}

async function refreshShuffleFromProxy() {
  try {
    const response = await fetch('/cmd/ytmd/shuffle/get', { headers: { Accept: 'application/json' } });
    const payload = await readCommandEnvelope(response);
    const data = payload?.data ?? payload;
    if (typeof data?.state === 'boolean') {
      playbackState.shuffle = data.state;
      updatePlaybackButtons();
    }
  } catch (err) {
    console.error('Failed to refresh shuffle state', err);
  }
}

async function seekToPosition(seconds) {
  const targetSeconds = Math.max(0, Math.floor(Number(seconds) || 0));
  playbackState.position = targetSeconds;
  updateProgressBar();
  const path = `/cmd/ytmd/seek-to/post?seconds=${targetSeconds}`;
  await sendPlayerCommand(path);
}

function setDockFolded(folded) {
  if (!dockEl) return;
  dockEl.classList.toggle('dock--folded', folded);
  if (dockFold) {
    dockFold.setAttribute('aria-expanded', folded ? 'false' : 'true');
    dockFold.setAttribute('aria-label', folded ? 'Unfold player' : 'Fold player');
  }
  try {
    localStorage.setItem(DOCK_FOLD_KEY, folded ? '1' : '0');
  } catch (_) {
    /* OBS / private mode */
  }
}

function initDockFold() {
  let folded = false;
  try {
    folded = localStorage.getItem(DOCK_FOLD_KEY) === '1';
  } catch (_) {
    folded = false;
  }
  setDockFolded(folded);
}

if (dockFold) {
  dockFold.addEventListener('click', () => {
    setDockFolded(!dockEl?.classList.contains('dock--folded'));
  });
}

if (compactPrev) {
  compactPrev.addEventListener('click', () => playerButtons.prev?.click());
}
if (compactNext) {
  compactNext.addEventListener('click', () => playerButtons.next?.click());
}
if (compactPlay) {
  compactPlay.addEventListener('click', () => {
    const isPlayingNow = playbackState.hasSong && !playbackState.isPaused;
    if (isPlayingNow) {
      playerButtons.pause?.click();
    } else {
      playerButtons.play?.click();
    }
  });
}

initDockFold();

Object.entries(playerButtons).forEach(([key, button]) => {
  if (!button) return;
  let path = '';
  switch (key) {
    case 'prev':
      path = '/cmd/ytmd/previous/post';
      break;
    case 'back':
      path = '/cmd/ytmd/go-back/post';
      break;
    case 'play':
      path = '/cmd/ytmd/play/post';
      break;
    case 'pause':
      path = '/cmd/ytmd/pause/post';
      break;
    case 'forward':
      path = '/cmd/ytmd/go-forward/post';
      break;
    case 'next':
      path = '/cmd/ytmd/next/post';
      break;
    case 'shuffle':
      path = '/cmd/ytmd/shuffle/post';
      break;
  }

  button.addEventListener('click', async () => {
    if (key === 'play') {
      setAdminPaused(false);
      notePlaybackProgress();
    } else if (key === 'pause') {
      window.clearTimeout(progressStallTimer);
      setAdminPaused(true);
    } else if (key === 'shuffle') {
      playbackState.shuffle = !playbackState.shuffle;
      updatePlaybackButtons();
    }

    if (key === 'back') {
      await seekToPosition(playbackState.position - 10);
      return;
    }
    if (key === 'forward') {
      await seekToPosition(playbackState.position + 10);
      return;
    }
    if (path) {
      await sendPlayerCommand(path);
    }
  });
});

if (songRequestForm) {
  songRequestForm.addEventListener('submit', (event) => {
    event.preventDefault();
    const picked = songRequestInputFromHighlight();
    submitSongRequest(picked || undefined, false);
  });
}
if (songRequestNow) {
  songRequestNow.addEventListener('click', () => {
    const picked = songRequestInputFromHighlight();
    submitSongRequest(picked || undefined, true);
  });
}
if (songSearchPopup) {
  songSearchPopup.addEventListener('mousedown', (event) => {
    event.preventDefault();
  });
  songSearchPopup.addEventListener('click', (event) => {
    const button = event.target.closest('[data-search-index]');
    if (!button) return;
    const item = songSearchItems[Number(button.dataset.searchIndex)];
    if (!item?.videoId) return;
    submitSongRequest(item.videoId, false);
  });
}
if (songRequestInput) {
  songRequestInput.addEventListener('input', scheduleSongSearch);
  songRequestInput.addEventListener('keydown', (event) => {
    if (event.key === 'ArrowDown' && songSearchItems.length) {
      event.preventDefault();
      const next = songSearchHighlight < songSearchItems.length - 1 ? songSearchHighlight + 1 : 0;
      setSongSearchHighlight(next);
      return;
    }
    if (event.key === 'ArrowUp' && songSearchItems.length) {
      event.preventDefault();
      const next = songSearchHighlight > 0 ? songSearchHighlight - 1 : songSearchItems.length - 1;
      setSongSearchHighlight(next);
      return;
    }
    if (event.key === 'Escape') {
      hideSongSearchPopup();
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      const picked = songRequestInputFromHighlight();
      submitSongRequest(picked || undefined, false);
    }
  });
  songRequestInput.addEventListener('blur', () => {
    window.setTimeout(hideSongSearchPopup, 120);
  });
}

progressBar.addEventListener('input', () => {
  playbackState.position = Number(progressBar.value) || 0;
  updateProgressBar();
});

progressBar.addEventListener('change', async () => {
  await seekToPosition(progressBar.value);
});

renderCommands();
loadSettings();
refreshChatCmdsPill();
fillCmdCooldownRows({});
refreshNowPlayingFromProxy();
refreshQueueFromProxy();
refreshShuffleFromProxy();
connectNowPlaying();
startAdminEvents();
window.addEventListener('resize', updateCurrentMetaMarquee);

const tabNav = document.querySelector('.tabs');
function fitAdminTabs() {
  if (!tabNav) return;
  tabNav.classList.remove('tabs--compact');
  const compact = tabNav.scrollWidth > tabNav.clientWidth + 1;
  tabNav.classList.toggle('tabs--compact', compact);
  tabs.forEach((tab) => {
    tab.title = compact ? (tab.querySelector('span')?.textContent || '') : '';
  });
}
if (tabNav && typeof ResizeObserver !== 'undefined') {
  new ResizeObserver(fitAdminTabs).observe(tabNav);
} else {
  window.addEventListener('resize', fitAdminTabs);
  fitAdminTabs();
}
window.addEventListener('focus', () => {
  if (!document.hidden) resumeAdminLive();
});
document.addEventListener('visibilitychange', () => {
  if (document.hidden) {
    pauseAdminLive();
  } else {
    resumeAdminLive();
  }
});
