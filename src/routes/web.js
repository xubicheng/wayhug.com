/**
 * Web 界面路由
 * 提供简洁的管理界面
 */

import { Router } from 'express';
import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';
import { getAllCharacters } from '../engine/index.js';
import { getCollection } from '../data/store.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const router = Router();

// 首页 - 控制台
router.get('/', (req, res) => {
  res.sendFile(join(__dirname, '..', '..', 'views', 'index.html'));
});

// 角色页面
router.get('/characters', (req, res) => {
  res.sendFile(join(__dirname, '..', '..', 'views', 'characters.html'));
});

// 内容列表页
router.get('/contents', (req, res) => {
  res.sendFile(join(__dirname, '..', '..', 'views', 'contents.html'));
});

// 内容详情页
router.get('/contents/:id', (req, res) => {
  res.sendFile(join(__dirname, '..', '..', 'views', 'content-detail.html'));
});

export default router;
