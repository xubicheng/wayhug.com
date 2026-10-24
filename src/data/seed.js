/**
 * 数据初始化脚本
 * 运行: npm run seed
 */

import { writeDB, readDB } from './store.js';
import { BUILTIN_CHARACTERS } from '../engine/character.js';
import { DIALOGUE_TEMPLATES, MOOD_MAP } from './templates.js';

const db = readDB();

// 写入角色
db.characters = BUILTIN_CHARACTERS.map(c => ({
  ...c,
  createdAt: new Date().toISOString()
}));

// 写入模板元数据
db.templates = Object.entries(DIALOGUE_TEMPLATES).map(([mood, lines]) => ({
  id: mood,
  mood,
  moodName: MOOD_MAP[mood] || mood,
  count: lines.length
}));

// 清空旧内容（可选）
db.contents = db.contents || [];

writeDB(db);

console.log('✅ 数据初始化完成');
console.log(`   角色: ${db.characters.length} 个`);
console.log(`   情绪模板: ${db.templates.length} 种`);
console.log(`   已有内容: ${db.contents.length} 条`);
