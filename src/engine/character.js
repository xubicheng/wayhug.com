/**
 * 角色系统 (Character / 动物层)
 * 拟人化动物角色，每个角色有独特的性格、语气词、说话风格
 */

// 内置角色定义
export const BUILTIN_CHARACTERS = [
  {
    id: 'fox',
    name: '小狐',
    species: '狐狸',
    emoji: '🦊',
    personality: '聪慧机敏，善于观察，喜欢用诗意的语言表达感受',
    tone: '温柔、略带感慨，喜欢引用自然意象',
    catchphrases: ['你看那...', '说来也巧，', '人生就像...'],
    styleTags: ['诗意', '感慨', '自然'],
    color: '#ff7e47'
  },
  {
    id: 'owl',
    name: '墨鸮',
    species: '猫头鹰',
    emoji: '🦉',
    personality: '博学沉稳，喜欢讲道理，话语中带着哲思',
    tone: '慢条斯理，喜欢用"老朽"自称，爱引经据典',
    catchphrases: ['老朽以为，', '古语有云，', '万物皆有其理。'],
    styleTags: ['哲理', '沉稳', '博学'],
    color: '#6b5b95'
  },
  {
    id: 'rabbit',
    name: '棉棉',
    species: '兔子',
    emoji: '🐰',
    personality: '活泼好奇，天真烂漫，对世界充满热情',
    tone: '欢快、充满活力，喜欢用感叹号和叠词',
    catchphrases: ['哇！', '好好玩呀！', '你知道吗？'],
    styleTags: ['活泼', '天真', '热情'],
    color: '#ffb6c1'
  },
  {
    id: 'bear',
    name: '憨熊',
    species: '熊',
    emoji: '🐻',
    personality: '憨厚朴实，话不多但句句实在，喜欢美食',
    tone: '慢吞吞，直来直去，偶尔冒出一句暖心话',
    catchphrases: ['俺觉得吧，', '饿了...', '慢慢来，不急。'],
    styleTags: ['朴实', '暖心', '实在'],
    color: '#8b6f47'
  },
  {
    id: 'cat',
    name: '喵先生',
    species: '猫',
    emoji: '🐱',
    personality: '优雅慵懒，带点傲娇，喜欢享受生活',
    tone: '慢条斯理，带点嘲讽但心地善良，爱用"哼"',
    catchphrases: ['哼，', '本喵觉得，', '无趣。'],
    styleTags: ['优雅', '傲娇', '慵懒'],
    color: '#708090'
  }
];

/**
 * 根据 ID 获取角色
 */
export function getCharacter(id) {
  return BUILTIN_CHARACTERS.find(c => c.id === id) || null;
}

/**
 * 获取所有角色
 */
export function getAllCharacters() {
  return BUILTIN_CHARACTERS;
}

/**
 * 获取角色的语气标签（用于台词生成）
 */
export function getCharacterTone(id) {
  const char = getCharacter(id);
  if (!char) return {};
  return {
    catchphrases: char.catchphrases,
    styleTags: char.styleTags,
    tone: char.tone,
    personality: char.personality
  };
}
