/**
 * 内核层 · TXT 解析器
 * 把一份纯文本故事解析为「唯一事实源」：标题、段落、场景、角色、情绪、扁平化配色。
 * 后续动画 / JSON / CSS / PDF 全部由这里派生，保证四套方案同源同貌。
 */

import { createHash } from 'node:crypto';

/** 解析规则版本：规则调整后用于自动重建演示作品 */
export const STORY_ENGINE_VERSION = 2;

export const ANIMALS = {
  fox: { name: '小狐', latin: 'fox', color: '#ff7e47', keywords: ['狐', '狸', '小狐', 'fox'] },
  owl: { name: '墨鸮', latin: 'owl', color: '#6b5b95', keywords: ['鸮', '墨鸮', '猫头鹰', 'owl', '夜', '月'] },
  rabbit: { name: '棉棉', latin: 'rabbit', color: '#ffb6c1', keywords: ['兔', '棉棉', 'rabbit', 'bunny'] },
  bear: { name: '憨熊', latin: 'bear', color: '#8b6f47', keywords: ['熊', '憨熊', 'bear'] },
  cat: { name: '喵先生', latin: 'cat', color: '#708090', keywords: ['猫', '喵', '喵先生', 'cat'] },
};

/** 情绪 → 扁平化色板（低饱和、纯色块，无渐变） */
export const MOODS = {
  joyful: { name: '明快', keywords: ['笑', '喜', '暖', '阳光', '欢', '跳', '唱', '甜', '春'], bg: '#fff4e0', ink: '#2d2a26', accent: '#ff9f43' },
  nostalgic: { name: '怀旧', keywords: ['曾', '旧', '回忆', '记得', '很久', '年', '童年', '故'], bg: '#eef1ff', ink: '#262a3d', accent: '#6b5b95' },
  thoughtful: { name: '沉思', keywords: ['想', '静', '问', '为什么', '也许', '忽然明白', '沉默'], bg: '#eaf7f0', ink: '#22332b', accent: '#2f9e77' },
  surprised: { name: '惊讶', keywords: ['突然', '忽然', '竟然', '惊', '没想到', '！'], bg: '#ffeef2', ink: '#3a2329', accent: '#e0567a' },
  melancholy: { name: '淡淡忧伤', keywords: ['雨', '冷', '别离', '告别', '孤单', '泪', '夜', '风'], bg: '#e8f1f5', ink: '#243440', accent: '#5d8aa8' },
  calm: { name: '平静', keywords: [], bg: '#f5f5f7', ink: '#2c3038', accent: '#708090' },
};

const SENTENCE_SPLIT = /([^。！？!?；;\n]+[。！？!?；;]?)/g;

function normalize(text) {
  return String(text || '')
    .replace(/\r\n?/g, '\n')
    .replace(/\u00a0/g, ' ')
    .replace(/[ \t]+/g, ' ')
    .trim();
}

/** 拆句：保留句末标点，过滤空句 */
export function splitSentences(text) {
  const out = [];
  const matches = normalize(text).match(SENTENCE_SPLIT) || [];
  for (const raw of matches) {
    const s = raw.trim();
    if (s) out.push(s);
  }
  return out;
}

/** 识别一句话里出现的动物：按出现次数降序、首次出现位置升序（决定本幕主角） */
function detectCast(text) {
  return Object.entries(ANIMALS)
    .map(([key, meta]) => {
      let count = 0;
      let first = Infinity;
      for (const k of meta.keywords) {
        let from = 0;
        let idx = text.indexOf(k, from);
        while (idx >= 0) {
          count += 1;
          first = Math.min(first, idx);
          from = idx + k.length;
          idx = text.indexOf(k, from);
        }
      }
      return { key, count, first };
    })
    .filter((x) => x.count > 0)
    .sort((a, b) => b.count - a.count || a.first - b.first)
    .map((x) => x.key);
}

/** 情绪判定：命中关键词数量最多者胜出，全部落空时用 calm */
function detectMood(text) {
  let best = 'calm';
  let bestScore = 0;
  for (const [key, meta] of Object.entries(MOODS)) {
    const score = meta.keywords.reduce((n, k) => (text.includes(k) ? n + 1 : n), 0);
    if (score > bestScore) {
      best = key;
      bestScore = score;
    }
  }
  return best;
}

/** 把段落重排为「一幕一画面」：过长段落按句子切块，单块不超过 maxChars */
function toScenes(paragraphs, maxChars = 96) {
  const scenes = [];
  for (const para of paragraphs) {
    if (para.length <= maxChars) {
      scenes.push(para);
      continue;
    }
    let buf = '';
    for (const sentence of splitSentences(para)) {
      if (buf && (buf + sentence).length > maxChars) {
        scenes.push(buf);
        buf = '';
      }
      buf += sentence;
      if (buf.length >= maxChars) {
        scenes.push(buf);
        buf = '';
      }
    }
    if (buf) scenes.push(buf);
  }
  return scenes.slice(0, 24); // 一页动画最多 24 幕，避免总时长失控
}

/**
 * 主解析函数
 * @param {string} txt 纯文本故事
 * @param {{title?: string, id?: string}} options
 */
export function parseStory(txt, options = {}) {
  const text = normalize(txt);
  if (!text) throw new Error('故事内容为空');

  const blocks = text
    .split(/\n\s*\n/)
    .map((b) => b.trim())
    .filter(Boolean);
  const paragraphs = blocks.length ? blocks : text.split('\n').map((l) => l.trim()).filter(Boolean);

  // 标题：优先使用传入标题；否则取首行短句当标题
  let title = (options.title || '').trim();
  let body = paragraphs;
  if (!title) {
    const first = paragraphs[0] || '';
    if (first.length <= 24 && !/[。！？!?]/.test(first)) {
      title = first;
      body = paragraphs.slice(1);
    } else {
      const s = splitSentences(first)[0] || first;
      title = s.replace(/[。！？!?]$/, '').slice(0, 16) + (s.length > 16 ? '…' : '');
    }
  }
  if (!body.length) body = paragraphs;

  const castKeys = [];
  for (const p of body) {
    for (const key of detectCast(p)) if (!castKeys.includes(key)) castKeys.push(key);
  }
  if (!castKeys.length) castKeys.push('fox');

  const sceneTexts = toScenes(body);
  let lastAnimal = castKeys[0];
  const scenes = sceneTexts.map((sceneText, i) => {
    const found = detectCast(sceneText);
    const animal = found[0] || lastAnimal;
    lastAnimal = animal;
    const mood = detectMood(sceneText);
    return {
      index: i,
      text: sceneText,
      animal,
      mood,
      palette: {
        bg: MOODS[mood].bg,
        ink: MOODS[mood].ink,
        accent: MOODS[mood].accent,
        animalColor: ANIMALS[animal].color,
      },
      sentences: splitSentences(sceneText),
    };
  });

  const cast = castKeys.map((key) => ({
    id: key,
    name: ANIMALS[key].name,
    color: ANIMALS[key].color,
    count: scenes.filter((s) => s.animal === key).length,
  }));

  const fingerprint = createHash('sha256').update(text).digest('hex').slice(0, 16);
  const moodCount = scenes.reduce((acc, s) => {
    acc[s.mood] = (acc[s.mood] || 0) + 1;
    return acc;
  }, {});

  return {
    id: options.id || `st_${fingerprint.slice(0, 8)}`,
    title,
    fingerprint,
    engineVersion: STORY_ENGINE_VERSION,
    createdAt: new Date().toISOString(),
    stats: {
      chars: text.length,
      sentences: splitSentences(text).length,
      scenes: scenes.length,
      cast: cast.length,
    },
    cast,
    scenes,
    mood: scenes[0]?.mood || 'calm',
    moodCount,
    text,
  };
}

/** 供 S3 下载：场景图 JSON（纯逻辑数据，不含渲染） */
export function storyToJSON(story) {
  return {
    meta: {
      engine: 'wayhug-lab/1.0',
      title: story.title,
      fingerprint: story.fingerprint,
      createdAt: story.createdAt,
      license: 'IP 体验权 · 非独占 · 免费试用',
    },
    architecture: [
      { layer: '内核层', role: 'TXT 纯文本 → 唯一事实源', artifact: 'story.json' },
      { layer: '引擎层', role: '角色/情绪/场景规则推理', artifact: 'scene graph' },
      { layer: '服务层', role: 'HTTP + 邮件指令服务', artifact: 'routes / imap' },
      { layer: '业务层', role: 'IP 体验权与交付流程', artifact: 'request / license' },
      { layer: '交付层', role: 'CSS 动画 + A4 证书', artifact: 'html / pdf' },
    ],
    cast: story.cast,
    scenes: story.scenes.map((s) => ({
      id: s.index,
      text: s.text,
      animal: s.animal,
      mood: s.mood,
      palette: s.palette,
    })),
    rules: [
      { when: '文本中出现「狐」', then: '角色 = fox' },
      { when: '文本中出现「兔」', then: '角色 = rabbit' },
      { when: '文本中出现「熊」', then: '角色 = bear' },
      { when: '文本中出现「猫/喵」', then: '角色 = cat' },
      { when: '文本中出现「鸮/月/夜」', then: '角色 = owl' },
      { when: '命中情绪关键词最多', then: '取该情绪色板' },
      { when: '段落超过 96 字符', then: '按句切分为多幕' },
    ],
    stats: story.stats,
  };
}
