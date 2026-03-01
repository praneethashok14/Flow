// Tauri 2.x globals — available because withGlobalTauri is true in tauri.conf.json.
// No bundler or npm required.
const { invoke } = window.__TAURI__.core;
const { listen }  = window.__TAURI__.event;

// ── State ─────────────────────────────────────────────────────────────────────

let tabs        = [];
let activeTabId = null;

const NARROW_THRESHOLD = 80; // px — below this, sidebar goes icon-only

// ── DOM refs ──────────────────────────────────────────────────────────────────

const sidebar      = document.getElementById('sidebar');
const tabList      = document.getElementById('tab-list');
const newTabBtn    = document.getElementById('new-tab-btn');
const backBtn      = document.getElementById('back-btn');
const forwardBtn   = document.getElementById('forward-btn');
const reloadBtn    = document.getElementById('reload-btn');
const addressBar   = document.getElementById('address-bar');
const contentArea  = document.getElementById('content-area');
const resizeHandle = document.getElementById('resize-handle');

// ── Content bounds sync ───────────────────────────────────────────────────────
// Reads the current bounding rect of #content-area (in CSS / logical pixels)
// and tells Rust to reposition all child webviews to match.

let boundsRafId = null;

function sendContentBounds() {
  // Throttle to once per animation frame so rapid events (drag, resize) don't
  // flood the IPC channel.
  if (boundsRafId !== null) return;
  boundsRafId = requestAnimationFrame(async () => {
    boundsRafId = null;
    const rect = contentArea.getBoundingClientRect();
    // Guard against zero-size rects during layout (e.g. before first paint).
    if (rect.width < 1 || rect.height < 1) return;
    try {
      await invoke('update_content_bounds', {
        bounds: {
          x:      rect.left,
          y:      rect.top,
          width:  rect.width,
          height: rect.height,
        },
      });
    } catch (err) {
      console.error('update_content_bounds failed:', err);
    }
  });
}

// ResizeObserver fires whenever #content-area's size changes — covers both
// window resizes and sidebar drags without needing two separate event handlers.
new ResizeObserver(sendContentBounds).observe(contentArea);

// ── Sidebar resize ────────────────────────────────────────────────────────────

let isResizing      = false;
let resizeStartX    = 0;
let resizeStartWidth = 0;

resizeHandle.addEventListener('mousedown', (e) => {
  isResizing       = true;
  resizeStartX     = e.clientX;
  resizeStartWidth = sidebar.getBoundingClientRect().width;
  resizeHandle.classList.add('dragging');
  document.body.style.cursor     = 'col-resize';
  document.body.style.userSelect = 'none';
  e.preventDefault();
});

document.addEventListener('mousemove', (e) => {
  if (!isResizing) return;
  const delta    = e.clientX - resizeStartX;
  const newWidth = Math.max(40, Math.min(480, resizeStartWidth + delta));
  sidebar.style.width = newWidth + 'px';
  document.documentElement.style.setProperty('--sidebar-width', newWidth + 'px');
  sidebar.classList.toggle('sidebar--narrow', newWidth < NARROW_THRESHOLD);
  // Bounds will be sent automatically by the ResizeObserver when the layout settles.
});

document.addEventListener('mouseup', () => {
  if (!isResizing) return;
  isResizing                     = false;
  document.body.style.cursor     = '';
  document.body.style.userSelect = '';
  resizeHandle.classList.remove('dragging');
});

// ── Tab rendering ─────────────────────────────────────────────────────────────

function renderTabs(tabData) {
  tabs        = tabData;
  activeTabId = tabData.find((t) => t.is_active)?.id ?? null;
  tabList.innerHTML = '';

  for (const tab of tabData) {
    const item        = document.createElement('div');
    item.className    = 'tab-item' + (tab.is_active ? ' active' : '');
    item.dataset.tabId = tab.id;

    item.appendChild(buildFaviconEl(tab));

    // Title — horizontal text, same orientation as the tab row, never rotated.
    const titleEl      = document.createElement('span');
    titleEl.className  = 'tab-title';
    titleEl.textContent = tab.title || tab.url;
    titleEl.title       = tab.title || tab.url;
    item.appendChild(titleEl);

    const closeBtn       = document.createElement('button');
    closeBtn.className   = 'tab-close';
    closeBtn.textContent = '×';
    closeBtn.title       = 'Close tab';
    closeBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      closeTab(tab.id);
    });
    item.appendChild(closeBtn);

    item.addEventListener('click', () => switchTab(tab.id));
    tabList.appendChild(item);
  }
}

function buildFaviconEl(tab) {
  if (tab.favicon_url) {
    const img    = document.createElement('img');
    img.className = 'tab-favicon';
    img.src       = tab.favicon_url;
    img.alt       = '';
    img.onerror   = () => img.replaceWith(makePlaceholder());
    return img;
  }
  return makePlaceholder();
}

function makePlaceholder() {
  const div     = document.createElement('div');
  div.className = 'tab-favicon-placeholder';
  return div;
}

// ── Tab operations ────────────────────────────────────────────────────────────

async function openNewTab() {
  const rect   = contentArea.getBoundingClientRect();
  const bounds = { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
  try {
    const updated = await invoke('new_tab', { bounds });
    renderTabs(updated);
  } catch (err) {
    console.error('new_tab failed:', err);
  }
}

async function closeTab(tabId) {
  try {
    const updated = await invoke('close_tab', { tabId });
    renderTabs(updated);
    const active = updated.find((t) => t.is_active);
    if (active) addressBar.value = active.url;
  } catch (err) {
    console.error('close_tab failed:', err);
  }
}

async function switchTab(tabId) {
  if (tabId === activeTabId) return;
  try {
    const updated = await invoke('switch_tab', { tabId });
    renderTabs(updated);
    const active = updated.find((t) => t.is_active);
    if (active) addressBar.value = active.url;
  } catch (err) {
    console.error('switch_tab failed:', err);
  }
}

// ── Navigation ────────────────────────────────────────────────────────────────

function normalizeUrl(raw) {
  const s = raw.trim();
  if (/^https?:\/\//i.test(s)) return s;
  if (/^[\w.-]+\.[a-z]{2,}(\/|$)/i.test(s)) return 'https://' + s;
  return 'https://duckduckgo.com/?q=' + encodeURIComponent(s);
}

addressBar.addEventListener('keydown', async (e) => {
  if (e.key !== 'Enter') return;
  const url = normalizeUrl(addressBar.value);
  addressBar.value = url;
  try {
    await invoke('navigate', { url });
  } catch (err) {
    console.error('navigate failed:', err);
  }
});

addressBar.addEventListener('focus', () => addressBar.select());

backBtn.addEventListener('click',    () => invoke('go_back').catch(console.error));
forwardBtn.addEventListener('click', () => invoke('go_forward').catch(console.error));
reloadBtn.addEventListener('click',  () => {
  const active = tabs.find((t) => t.is_active);
  if (active) invoke('navigate', { url: active.url }).catch(console.error);
});

newTabBtn.addEventListener('click', openNewTab);

// ── Backend events ────────────────────────────────────────────────────────────

listen('tabs-updated', (event) => {
  renderTabs(event.payload);
  // Re-sync bounds every time a tab is created/switched — Rust positions new
  // webviews using estimated layout values; this corrects them immediately.
  sendContentBounds();
});

listen('tab-title-changed', (event) => {
  const { tab_id, title } = event.payload;
  const tab = tabs.find((t) => t.id === tab_id);
  if (tab) {
    tab.title = title;
    const el = tabList.querySelector(`[data-tab-id="${tab_id}"] .tab-title`);
    if (el) { el.textContent = title; el.title = title; }
  }
  if (tab_id === activeTabId) document.title = title + ' — Flow';
});

listen('navigation-committed', (event) => {
  const { tab_id, url } = event.payload;
  const tab = tabs.find((t) => t.id === tab_id);
  if (tab) tab.url = url;
  if (tab_id === activeTabId) addressBar.value = url;
});

// ── Boot ──────────────────────────────────────────────────────────────────────
// JS owns initial tab creation so that bounds always come from the real
// getBoundingClientRect() — eliminating the coordinate-system mismatch that
// caused the webview to overlap the nav bar when Rust guessed the layout.

(async () => {
  const initial = await invoke('get_tabs').catch(() => []);

  if (initial.length > 0) {
    // App restarted with existing state — restore tabs and correct bounds.
    renderTabs(initial);
    const active = initial.find((t) => t.is_active);
    if (active) addressBar.value = active.url;
    sendContentBounds();
  } else {
    // First launch — open the home tab now that CSS layout is ready.
    await openNewTab();
  }
})();
