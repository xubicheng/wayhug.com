/**
 * API 路由层
 * RESTful 接口，供网站前端 / 移动端调用
 */

import { Router } from 'express';
import { getAllCharacters, getCharacter, generateDialogue, generateDialogueRound, generateArticle } from '../engine/index.js';
import { getCollection, findById, insert, genId, remove } from '../data/store.js';

const router = Router();

// ===== 角色接口 =====

// 获取所有角色
router.get('/characters', (req, res) => {
  res.json({ code: 0, data: getAllCharacters() });
});

// 获取单个角色
router.get('/characters/:id', (req, res) => {
  const char = getCharacter(req.params.id);
  if (!char) return res.status(404).json({ code: 404, message: '角色不存在' });
  res.json({ code: 0, data: char });
});

// ===== 台词生成接口 =====

// 生成单条台词
router.post('/dialogue/generate', (req, res) => {
  try {
    const { characterId, topic, mood } = req.body;
    if (!characterId || !topic) {
      return res.status(400).json({ code: 400, message: '缺少参数 characterId 或 topic' });
    }
    const result = generateDialogue({ characterId, topic, mood });

    // 存入数据库
    insert('contents', {
      ...result,
      type: 'dialogue',
      savedAt: new Date().toISOString()
    });

    res.json({ code: 0, data: result });
  } catch (err) {
    res.status(500).json({ code: 500, message: err.message });
  }
});

// 生成多角色对话
router.post('/dialogue/round', (req, res) => {
  try {
    const { characterIds, topic, mood } = req.body;
    if (!characterIds || !Array.isArray(characterIds) || characterIds.length === 0) {
      return res.status(400).json({ code: 400, message: '缺少参数 characterIds（数组）' });
    }
    if (!topic) {
      return res.status(400).json({ code: 400, message: '缺少参数 topic' });
    }
    const results = generateDialogueRound({ characterIds, topic, mood });
    res.json({ code: 0, data: results });
  } catch (err) {
    res.status(500).json({ code: 500, message: err.message });
  }
});

// 生成文章
router.post('/article/generate', (req, res) => {
  try {
    const { characterId, topic, mood, paragraphs } = req.body;
    if (!characterId || !topic) {
      return res.status(400).json({ code: 400, message: '缺少参数 characterId 或 topic' });
    }
    const result = generateArticle({ characterId, topic, mood, paragraphs });

    insert('contents', {
      id: genId('art'),
      ...result,
      type: 'article',
      savedAt: new Date().toISOString()
    });

    res.json({ code: 0, data: result });
  } catch (err) {
    res.status(500).json({ code: 500, message: err.message });
  }
});

// ===== 内容管理接口 =====

// 获取已生成内容列表
router.get('/contents', (req, res) => {
  const { type, limit = 20 } = req.query;
  let contents = getCollection('contents');
  if (type) contents = contents.filter(c => c.type === type);
  contents = contents.slice(-Number(limit)).reverse();
  res.json({ code: 0, data: contents });
});

// 获取单条内容
router.get('/contents/:id', (req, res) => {
  const content = findById('contents', req.params.id);
  if (!content) return res.status(404).json({ code: 404, message: '内容不存在' });
  res.json({ code: 0, data: content });
});

// 删除内容
router.delete('/contents/:id', (req, res) => {
  const ok = remove('contents', req.params.id);
  res.json({ code: ok ? 0 : 404, message: ok ? '已删除' : '内容不存在' });
});

// ===== 引擎信息 =====

router.get('/info', (req, res) => {
  res.json({
    code: 0,
    data: {
      name: 'Wayhug AI Engine',
      version: '0.1.0',
      engine: 'rule-based-v1',
      characters: getAllCharacters().length,
      license: 'MIT'
    }
  });
});

export default router;
