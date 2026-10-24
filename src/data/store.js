/**
 * 数据存储层 (Data Layer)
 * 基于 JSON 文件的轻量级存储，MVP 阶段零配置
 * 后续可替换为 SQLite / PostgreSQL
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DATA_DIR = join(__dirname, '..', '..', 'data');
const DB_FILE = join(DATA_DIR, 'db.json');

// 确保 data 目录存在
if (!existsSync(DATA_DIR)) {
  mkdirSync(DATA_DIR, { recursive: true });
}

// 默认空数据库结构
const DEFAULT_DB = {
  characters: [],
  contents: [],
  templates: [],
  meta: {
    createdAt: new Date().toISOString(),
    version: '0.1.0'
  }
};

/**
 * 读取整个数据库
 */
export function readDB() {
  if (!existsSync(DB_FILE)) {
    writeFileSync(DB_FILE, JSON.stringify(DEFAULT_DB, null, 2), 'utf-8');
    return structuredClone(DEFAULT_DB);
  }
  try {
    const raw = readFileSync(DB_FILE, 'utf-8');
    return JSON.parse(raw);
  } catch (err) {
    console.error('[store] 读取数据库失败，使用默认结构:', err.message);
    return structuredClone(DEFAULT_DB);
  }
}

/**
 * 写入整个数据库
 */
export function writeDB(db) {
  db.meta.updatedAt = new Date().toISOString();
  writeFileSync(DB_FILE, JSON.stringify(db, null, 2), 'utf-8');
  return db;
}

/**
 * 获取指定集合
 */
export function getCollection(name) {
  const db = readDB();
  return db[name] || [];
}

/**
 * 通过 ID 获取单条记录
 */
export function findById(collection, id) {
  const items = getCollection(collection);
  return items.find(item => item.id === id) || null;
}

/**
 * 插入新记录
 */
export function insert(collection, record) {
  const db = readDB();
  if (!db[collection]) db[collection] = [];
  db[collection].push(record);
  writeDB(db);
  return record;
}

/**
 * 更新记录
 */
export function update(collection, id, patch) {
  const db = readDB();
  const items = db[collection] || [];
  const idx = items.findIndex(item => item.id === id);
  if (idx === -1) return null;
  items[idx] = { ...items[idx], ...patch, updatedAt: new Date().toISOString() };
  db[collection] = items;
  writeDB(db);
  return items[idx];
}

/**
 * 删除记录
 */
export function remove(collection, id) {
  const db = readDB();
  const items = db[collection] || [];
  const filtered = items.filter(item => item.id !== id);
  db[collection] = filtered;
  writeDB(db);
  return items.length !== filtered.length;
}

/**
 * 生成唯一 ID
 */
export function genId(prefix = 'id') {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}
