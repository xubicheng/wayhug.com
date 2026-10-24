/**
 * 台词模板库 (Dialogue Templates)
 * 按主题 + 情绪分类，逻辑引擎会根据角色性格 + 模板生成台词
 */

export const DIALOGUE_TEMPLATES = {
  // 情绪：感慨
  nostalgic: [
    '时光如白驹过隙，{topic}已悄然变换了模样。',
    '每当想起{topic}，心中便涌起一股说不清的滋味。',
    '{topic}啊，你承载了多少旧日的时光。',
    '站在{topic}面前，仿佛听见岁月在低声诉说。'
  ],
  // 情绪：喜悦
  joyful: [
    '哇！{topic}真是太棒了！',
    '一想到{topic}，心情就像开了花一样！',
    '{topic}让人忍不住嘴角上扬呢！',
    '今天的{topic}格外美好，让人满心欢喜！'
  ],
  // 情绪：思考
  thoughtful: [
    '关于{topic}，我想了很久，也许...',
    '{topic}这件事，值得我们细细品味。',
    '老朽以为，{topic}之中藏着深意。',
    '不妨静下心来，感受{topic}带来的启示。'
  ],
  // 情绪：温暖
  warm: [
    '{topic}就像冬日里的一杯热茶，暖到了心里。',
    '有{topic}陪伴的日子，总是那么安心。',
    '{topic}让这个世界变得柔软了一些。',
    '每当疲惫时，{topic}总能给我力量。'
  ],
  // 情绪：好奇
  curious: [
    '{topic}？这倒是有趣，让我好好看看。',
    '咦？{topic}背后藏着什么秘密呢？',
    '关于{topic}，我有好多问题想问！',
    '{topic}真是个谜一样的存在啊。'
  ]
};

// 情绪到中文的映射
export const MOOD_MAP = {
  nostalgic: '感慨',
  joyful: '喜悦',
  thoughtful: '思考',
  warm: '温暖',
  curious: '好奇'
};

// 支持的主题关键词，用于自动匹配情绪
export const TOPIC_MOOD_HINTS = {
  '秋天|落叶|秋风|黄昏|旧时光|回忆': 'nostalgic',
  '春天|花开|阳光|生日|成功|旅行': 'joyful',
  '人生|意义|选择|未来|梦想|读书': 'thoughtful',
  '家|朋友|陪伴|拥抱|茶|阳光': 'warm',
  '新|奇怪|秘密|发现|探索|未知': 'curious'
};

/**
 * 根据主题猜测情绪
 */
export function guessMood(topic) {
  for (const [pattern, mood] of Object.entries(TOPIC_MOOD_HINTS)) {
    const regex = new RegExp(pattern);
    if (regex.test(topic)) return mood;
  }
  return 'thoughtful'; // 默认思考
}

/**
 * 从数组中随机取一个元素
 */
export function randomPick(arr) {
  return arr[Math.floor(Math.random() * arr.length)];
}
