/**
 * 逻辑引擎 (Logic Engine)
 * 核心 AI 推理模块 — MVP 阶段基于规则/模板生成台词
 * 设计为可扩展：后续可接入真实 LLM（OpenAI / Claude / 国产大模型）
 *
 * 生成流程：
 *   输入(characterId, topic, mood)
 *     → 加载角色性格 & 语气
 *     → 匹配情绪模板
 *     → 注入角色口头禅 & 风格
 *     → 组装完整台词
 *     → 输出 { text, character, mood, topic, meta }
 */

import { getCharacter, getCharacterTone } from './character.js';
import { DIALOGUE_TEMPLATES, MOOD_MAP, guessMood, randomPick } from '../data/templates.js';

/**
 * 台词生成器（规则引擎版）
 * @param {Object} params
 * @param {string} params.characterId - 角色 ID
 * @param {string} params.topic - 主题
 * @param {string} [params.mood] - 情绪（可选，不填则自动猜测）
 * @returns {Object} 生成结果
 */
export function generateDialogue({ characterId, topic, mood }) {
  const character = getCharacter(characterId);
  if (!character) {
    throw new Error(`角色不存在: ${characterId}`);
  }

  // 自动猜测情绪
  const resolvedMood = mood && DIALOGUE_TEMPLATES[mood] ? mood : guessMood(topic);
  const templates = DIALOGUE_TEMPLATES[resolvedMood] || DIALOGUE_TEMPLATES.thoughtful;

  // 选取基础模板并填入主题
  let baseText = randomPick(templates).replace(/\{topic\}/g, topic);

  // 注入角色风格
  const tone = getCharacterTone(characterId);
  baseText = injectCharacterStyle(baseText, tone, resolvedMood);

  // 组装完整台词
  const opening = randomPick(tone.catchphrases);
  const fullText = `${opening}${baseText}`;

  return {
    id: `dlg_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`,
    text: fullText,
    character: {
      id: character.id,
      name: character.name,
      emoji: character.emoji,
      species: character.species
    },
    topic,
    mood: resolvedMood,
    moodName: MOOD_MAP[resolvedMood] || resolvedMood,
    engine: 'rule-based-v1',
    createdAt: new Date().toISOString()
  };
}

/**
 * 注入角色说话风格
 */
function injectCharacterStyle(text, tone, mood) {
  let result = text;

  // 根据角色风格标签做微调
  const tags = tone.styleTags || [];

  if (tags.includes('诗意')) {
    // 诗意角色：句尾加轻柔意象
    if (Math.random() > 0.4) {
      const imagery = ['，如风过竹林', '，似水流年', '，恰似月光洒落', '，宛如初见'];
      result = result.replace(/[。！？]$/, m => m === '。' ? randomPick(imagery) + '。' : m);
    }
  }

  if (tags.includes('哲理')) {
    // 哲理角色：可能追加一句总结
    if (Math.random() > 0.5) {
      const wisdom = [' 这便是万物的规律。', ' 存在即合理。', ' 一切自有安排。'];
      result = result.replace(/[。！？]$/, '') + randomPick(wisdom);
    }
  }

  if (tags.includes('活泼')) {
    // 活泼角色：加感叹号或语气词
    if (Math.random() > 0.4) {
      result = result.replace(/[。]$/, '呀！');
    }
  }

  if (tags.includes('朴实')) {
    // 朴实角色：简化修辞
    result = result.replace(/[，,]{2,}/g, '，').replace(/[。]{2,}/g, '。');
  }

  if (tags.includes('傲娇')) {
    // 傲娇角色：加转折
    if (Math.random() > 0.5) {
      result = result.replace(/[。！？]$/, '，哼。');
    }
  }

  return result;
}

/**
 * 生成多轮对话（多个角色围绕同一主题）
 */
export function generateDialogueRound({ characterIds, topic, mood }) {
  const moodResolved = mood || guessMood(topic);
  return characterIds.map(id => generateDialogue({ characterId: id, topic, mood: moodResolved }));
}

/**
 * 生成博客文章段落（长文本生成 MVP）
 */
export function generateArticle({ characterId, topic, mood, paragraphs = 3 }) {
  const lines = [];
  for (let i = 0; i < paragraphs; i++) {
    const dlg = generateDialogue({ characterId, topic, mood });
    lines.push(dlg.text);
  }
  return {
    title: `${topic} · ${getCharacter(characterId)?.name || ''}的随笔`,
    paragraphs: lines,
    character: getCharacter(characterId),
    topic,
    createdAt: new Date().toISOString()
  };
}
