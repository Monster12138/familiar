import { applyTranslations, t } from './i18n.js';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

const STATES = ['idle','working','thinking','interacting','celebrating','alarmed','sleeping','watching','happy'];
const fields = ['name','author','version','email','license','source','description'];
const $ = id => document.getElementById(id);
let lang = 'zh-CN', packs = [], assets = {}, editingId = null, dirty = false, toastTimer;
let createdAt = null;
let selectedPackId = null;
let selectedState = 'idle';
const previewUrls = new Map();
const installedUrls = new Map();
let discardResolve = null;
let draggedState = null;
const stateLabel = key => t(`state_${key === 'happy' ? 'interacting' : key}`, lang);
function toast(message, type = '') { const node = $('toast'); node.textContent = message; node.className = `show ${type}`; clearTimeout(toastTimer); toastTimer = setTimeout(() => node.className = '', 3500); }
function markDirty() { dirty = true; }
function setStateAsset(key, path) {
  assets[key] = path;
  selectedState = key;
  markDirty();
  renderStates();
  renderPreview();
}
function highlightDropState(key) {
  if (draggedState === key) return;
  document.querySelector('.state-row.drag-over')?.classList.remove('drag-over');
  draggedState = key;
  if (key) document.querySelector(`.state-row[data-state="${key}"]`)?.classList.add('drag-over');
}
function stateAtDropPosition(position) {
  // Wry's macOS drop callback uses Cocoa view coordinates (points), even though Tauri labels them physical.
  const scale = /Macintosh|Mac OS X/.test(navigator.userAgent) ? 1 : window.devicePixelRatio;
  const row = document.elementFromPoint(position.x / scale, position.y / scale)?.closest('.state-row');
  return row?.dataset.state || null;
}
async function imageUrl(path) {
  if (!path) return '';
  if (installedUrls.has(path)) return installedUrls.get(path);
  if (previewUrls.has(path)) return previewUrls.get(path);
  const bytes = await invoke('read_workshop_image', {path});
  const extension = path.split('.').pop().toLowerCase();
  const mime = {png:'image/png',apng:'image/png',gif:'image/gif',webp:'image/webp',svg:'image/svg+xml'}[extension] || 'application/octet-stream';
  const url = URL.createObjectURL(new Blob([new Uint8Array(bytes)], {type:mime}));
  previewUrls.set(path, url);
  return url;
}
function renderStates() {
  $('states').replaceChildren();
  for (const key of STATES) {
    const row = document.createElement('div'); row.className = `state-row ${selectedState === key ? 'selected' : ''}`; row.dataset.state = key;
    row.tabIndex = 0;
    row.setAttribute('role', 'button');
    row.setAttribute('aria-pressed', String(selectedState === key));
    row.onclick = () => { selectedState = key; renderStates(); renderPreview(); };
    row.onkeydown = event => { if (event.target === row && (event.key === 'Enter' || event.key === ' ')) { event.preventDefault(); row.click(); } };
    const img = document.createElement('img'); img.alt = '';
    if (assets[key]) imageUrl(assets[key]).then(url => img.src = url).catch(() => img.alt = '×');
    const label = document.createElement('span'); label.className = 'state-name'; label.textContent = stateLabel(key) + (key === 'idle' ? ' *' : '');
    const file = document.createElement('span'); file.className = 'file-name'; file.textContent = assets[key]?.split(/[\\/]/).pop() || '—'; file.title = assets[key] || '';
    const choose = document.createElement('button'); choose.textContent = t('workshop_choose', lang);
    choose.onclick = async event => { event.stopPropagation(); try { const path = await invoke('pick_workshop_image'); if (path) setStateAsset(key, path); } catch (e) { toast(String(e), 'error'); } };
    const remove = document.createElement('button'); remove.textContent = t('workshop_remove', lang); remove.disabled = !assets[key];
    remove.onclick = event => { event.stopPropagation(); delete assets[key]; markDirty(); renderStates(); renderPreview(); };
    row.append(img,label,file,choose,remove); $('states').append(row);
  }
}
function renderPreview() {
  const selected = selectedState;
  const path = assets[selected];
  $('preview-image').removeAttribute('src');
  if (path) imageUrl(path).then(url => { if (selectedState === selected && assets[selected] === path) $('preview-image').src = url; }).catch(e => toast(String(e), 'error'));
  $('preview-state-name').textContent = stateLabel(selected);
  $('preview-file').textContent = path ? path.split(/[\\/]/).pop() : t('workshop_no_image', lang);
}
function confirmAction(message, actionText) {
  if (discardResolve) return Promise.resolve(false);
  $('discard-title').textContent = message;
  $('discard-confirm').textContent = actionText;
  $('discard-overlay').hidden = false;
  $('discard-cancel').focus();
  return new Promise(resolve => { discardResolve = resolve; });
}
function confirmDiscard() {
  if (!dirty) return Promise.resolve(true);
  return confirmAction(t('workshop_discard', lang), t('workshop_discard_action', lang));
}
function finishDiscard(answer) {
  $('discard-overlay').hidden = true;
  $('discard-title').textContent = t('workshop_discard', lang);
  $('discard-confirm').textContent = t('workshop_discard_action', lang);
  const resolve = discardResolve;
  discardResolve = null;
  resolve?.(answer);
}
function releasePreviewUrls() {
  for (const url of previewUrls.values()) URL.revokeObjectURL(url);
  previewUrls.clear();
}
function reset() {
  releasePreviewUrls(); installedUrls.clear();
  editingId = null; createdAt = null; selectedPackId = null; selectedState = 'idle'; assets = {}; dirty = false;
  for (const key of fields) $(key).value = '';
  $('version').value = '1.0.0'; renderStates(); renderPreview(); renderPacks();
}
async function loadPack(pack, copy) {
  if (!await confirmDiscard()) return;
  releasePreviewUrls();
  editingId = copy ? null : pack.manifest.id; createdAt = copy ? null : pack.manifest.created_at; assets = {};
  selectedPackId = pack.manifest.id;
  installedUrls.clear();
  for (const [key,file] of Object.entries(pack.manifest.states || {})) {
    const path = `${pack.path}/${file}`;
    assets[key] = path;
    installedUrls.set(path, pack.is_builtin ? `/sprites/${pack.manifest.id}/${file}` : convertFileSrc(path));
  }
  for (const key of fields) $(key).value = pack.manifest[key] || '';
  if (copy) $('name').value += ` ${t('workshop_copy_suffix', lang)}`;
  selectedState = STATES.find(key => assets[key]) || 'idle';
  dirty = false; renderStates(); renderPreview(); renderPacks();
}
function renderPacks() {
  $('packs').replaceChildren();
  for (const pack of packs) {
    const row = document.createElement('div'); row.className = `pack-option ${selectedPackId === pack.manifest.id ? 'active' : ''}`;
    const select = document.createElement('button'); select.className = 'pack-row';
    select.textContent = pack.manifest.name;
    select.onclick = () => { void loadPack(pack, pack.is_builtin); };
    row.append(select);
    if (!pack.is_builtin) {
      const remove = document.createElement('button');
      remove.className = 'pack-delete';
      remove.title = t('btn_delete_pack', lang);
      remove.setAttribute('aria-label', `${t('btn_delete_pack', lang)} ${pack.manifest.name}`);
      remove.innerHTML = '<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 6h18M8 6V4h8v2m3 0-1 14H6L5 6m4 4v6m6-6v6"/></svg>';
      remove.onclick = () => { void deletePack(pack); };
      row.append(remove);
    }
    $('packs').append(row);
  }
}
async function deletePack(pack) {
  const hasUnsavedEdits = selectedPackId === pack.manifest.id && dirty;
  const messageKey = hasUnsavedEdits ? 'workshop_delete_confirm_dirty' : 'workshop_delete_confirm';
  const message = t(messageKey, lang).replace('{name}', pack.manifest.name);
  if (!await confirmAction(message, t('workshop_delete_action', lang))) return;
  try {
    await invoke('delete_sprite_pack', {id: pack.manifest.id});
    packs = packs.filter(item => item.manifest.id !== pack.manifest.id);
    if (selectedPackId === pack.manifest.id) reset();
    renderPacks();
    if (selectedPackId === null && packs.length) await loadPack(packs[0], packs[0].is_builtin);
    toast(t('workshop_delete_success', lang), 'success');
  } catch (error) {
    toast(t('workshop_delete_failed', lang) + String(error), 'error');
  }
}
async function refreshPacks() { packs = await invoke('get_sprite_packs'); renderPacks(); }
function manifest() {
  const data = Object.fromEntries(fields.map(key => [key, $(key).value.trim()]));
  return {format_version: 1, ...data, id: editingId || '', license: data.license || null, source: data.source || null,
    created_at: createdAt || new Date().toISOString().slice(0,10), preview: null, states: {}, sha256: null};
}
async function save() {
  try {
    document.querySelectorAll('.invalid').forEach(node => node.classList.remove('invalid'));
    for (const [state, path] of Object.entries(assets)) {
      const url = await imageUrl(path);
      await new Promise((resolve, reject) => {
        const image = new Image();
        image.onload = resolve;
        image.onerror = () => reject(`${state}: ${t('workshop_invalid_image', lang)}`);
        image.src = url;
      });
    }
    const data = manifest();
    const info = await invoke('save_workshop_pack', {draft: {manifest: data, assets, editing_id: editingId}});
    editingId = info.manifest.id; createdAt = info.manifest.created_at;
    selectedPackId = info.manifest.id;
    releasePreviewUrls();
    installedUrls.clear();
    for (const [key,file] of Object.entries(info.manifest.states)) {
      const path = `${info.path}/${file}`;
      assets[key] = path;
      installedUrls.set(path, convertFileSrc(path) + `?v=${encodeURIComponent(info.asset_revision)}`);
    }
    dirty = false; await refreshPacks(); renderStates(); renderPreview(); toast(t('workshop_saved', lang), 'success');
    return true;
  } catch (e) {
    const key = /^([a-z_]+):/.exec(String(e))?.[1];
    const target = $(key) || document.querySelector(`[data-state="${key}"]`);
    if (target) { target.classList.add('invalid'); target.scrollIntoView({block:'nearest'}); }
    toast(String(e), 'error'); return false;
  }
}
async function exportPack() {
  if (dirty || !editingId) { if (!await save()) return; }
  try { const path = await invoke('export_workshop_pack', {id: editingId}); if (path) toast(t('workshop_exported', lang), 'success'); }
  catch (e) { toast(String(e), 'error'); }
}

async function init() {
  try { const config = await invoke('get_config'); lang = config.general?.language || lang; } catch (_) { /* defaults to Chinese */ }
  applyTranslations(lang); document.documentElement.lang = lang.startsWith('zh') ? 'zh' : 'en';
  $('new').onclick = async () => { if (await confirmDiscard()) reset(); };
  $('discard-cancel').onclick = () => finishDiscard(false);
  $('discard-confirm').onclick = () => finishDiscard(true);
  $('save').onclick = save; $('export').onclick = exportPack;
  $('discard-overlay').addEventListener('click', event => { if (event.target === $('discard-overlay')) finishDiscard(false); });
  for (const key of fields) $(key).addEventListener('input', markDirty);
  await getCurrentWebviewWindow().onCloseRequested(event => {
    if (!dirty) return;
    event.preventDefault();
    void confirmDiscard()
      .then(confirmed => { if (confirmed) return getCurrentWebviewWindow().destroy(); })
      .catch(error => toast(String(error), 'error'));
  });
  const webview = getCurrentWebview();
  await webview.onDragDropEvent(event => {
    const {type, position, paths} = event.payload;
    if (type === 'leave') { highlightDropState(null); return; }
    const key = stateAtDropPosition(position);
    if (type === 'enter' || type === 'over') { highlightDropState(key); return; }
    highlightDropState(null);
    if (!key || type !== 'drop') return;
    if (paths.length !== 1) { toast(t('workshop_drop_one', lang), 'error'); return; }
    const path = paths[0];
    if (!/\.(png|apng|gif|webp|svg)$/i.test(path)) { toast(t('workshop_drop_format', lang), 'error'); return; }
    void imageUrl(path).then(() => setStateAsset(key, path)).catch(error => toast(String(error), 'error'));
  });
  await listen('sprite_pack_imported', refreshPacks);
  await listen('sprite_pack_deleted', refreshPacks);
  window.addEventListener('beforeunload', releasePreviewUrls);
  reset(); await refreshPacks();
  if (packs.length) await loadPack(packs[0], packs[0].is_builtin);
}
init().catch(e => toast(String(e), 'error'));
