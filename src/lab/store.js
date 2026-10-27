/**
 * 数据层 · 实验场存储（data/lab.json）
 * 与原有 data/db.json 完全隔离，四套方案共用：故事、体验申请、邮件去重。
 */

import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { randomBytes } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { parseStory, STORY_ENGINE_VERSION } from './story.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DATA_DIR = join(__dirname, '..', '..', 'data');
const FILE = join(DATA_DIR, 'lab.json');
const DEMO_FILE = join(__dirname, '..', '..', 'stories', 'demo.txt');

const EMPTY = { stories: [], requests: [], seen: [], counter: 0 };

let db = null;

function load() {
  if (db) return db;
  try {
    if (existsSync(FILE)) {
      db = { ...EMPTY, ...JSON.parse(readFileSync(FILE, 'utf8')) };
    } else {
      db = structuredClone(EMPTY);
    }
  } catch (err) {
    console.warn('[lab] 读取 lab.json 失败，使用空库:', err.message);
    db = structuredClone(EMPTY);
  }
  return db;
}

function persist() {
  try {
    if (!existsSync(DATA_DIR)) mkdirSync(DATA_DIR, { recursive: true });
    const tmp = `${FILE}.tmp`;
    writeFileSync(tmp, JSON.stringify(db, null, 2), 'utf8');
    renameSync(tmp, FILE);
    return true;
  } catch (err) {
    // 只读文件系统（如沙箱预览）不应拖垮服务：数据留在内存，接口照常响应
    console.warn('[lab] 写入 lab.json 失败（数据仅保存在内存）:', err.message);
    return false;
  }
}

function certId() {
  const d = new Date();
  const day = `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}`;
  return `WH-${day}-${randomBytes(2).toString('hex').toUpperCase()}`;
}

export function saveStory(story, extra = {}) {
  const data = load();
  const record = { ...story, ...extra };
  const idx = data.stories.findIndex((s) => s.id === record.id);
  if (idx >= 0) data.stories[idx] = record;
  else data.stories.unshift(record);
  data.stories = data.stories.slice(0, 200);
  persist();
  return record;
}

export function getStory(id) {
  return load().stories.find((s) => s.id === id) || null;
}

export function listStories(limit = 30) {
  return load()
    .stories.slice(0, limit)
    .map(({ text, ...rest }) => rest); // 列表不返回全文，减小体积
}

export function createRequest(fields) {
  const data = load();
  const now = new Date();
  const record = {
    id: certId(),
    createdAt: now.toISOString(),
    expiresAt: new Date(now.getTime() + 90 * 24 * 3600 * 1000).toISOString(),
    status: 'pending',
    reply: null,
    ...fields,
  };
  data.requests.unshift(record);
  data.requests = data.requests.slice(0, 500);
  data.counter += 1;
  persist();
  return record;
}

export function getRequest(id) {
  return load().requests.find((r) => r.id === id) || null;
}

export function updateRequest(id, patch) {
  const data = load();
  const r = data.requests.find((x) => x.id === id);
  if (!r) return null;
  Object.assign(r, patch);
  persist();
  return r;
}

export function listRequests(limit = 100) {
  return load().requests.slice(0, limit);
}

/** 邮件去重：返回 true 表示首次见到 */
export function isNewMail(messageId) {
  const data = load();
  const key = String(messageId || '').slice(0, 200);
  if (!key) return true;
  if (data.seen.includes(key)) return false;
  data.seen.unshift(key);
  data.seen = data.seen.slice(0, 500);
  persist();
  return true;
}

export function stats() {
  const data = load();
  return {
    stories: data.stories.length,
    requests: data.requests.length,
    pending: data.requests.filter((r) => r.status !== 'replied').length,
    replied: data.requests.filter((r) => r.status === 'replied').length,
  };
}

/** 首次启动时把 stories/demo.txt 解析为常驻演示作品（S1 使用） */
export function ensureDemo() {
  const data = load();
  const existing = data.stories.find((s) => s.id === 'demo');
  if (existing && existing.engineVersion === STORY_ENGINE_VERSION) return existing;
  if (!existsSync(DEMO_FILE)) return existing || null;
  const txt = readFileSync(DEMO_FILE, 'utf8');
  const story = parseStory(txt, { id: 'demo', title: '' });
  return saveStory(story, { demo: true, source: 'demo', asset: 'stories/demo.txt' });
}
