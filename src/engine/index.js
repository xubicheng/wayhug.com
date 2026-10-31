/**
 * AI 引擎入口 (Engine Layer)
 * 统一导出引擎能力：角色、逻辑、数据
 */

export * from './character.js';
export * from './logic.js';
export { getAllCharacters, getCharacter } from './character.js';
export { generateDialogue, generateDialogueRound, generateArticle } from './logic.js';
