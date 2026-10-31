/**
 * 实验场自检：TXT 解析 → 动画三件套 → A4 证书
 * 运行：npm run lab:selftest
 */
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseStory, storyToJSON } from '../src/lab/story.js';
import { buildBundle } from '../src/lab/animate.js';
import { certificateBuffer } from '../src/lab/certificate.js';
import { resolveFont } from '../src/lab/font.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');
const txt = readFileSync(join(root, 'stories', 'demo.txt'), 'utf8');

const story = parseStory(txt, { id: 'demo' });
console.log(`[1/3] 解析：${story.title} · ${story.stats.scenes} 幕 · ${story.stats.chars} 字符 · cast=${story.cast.map((c) => c.id).join(',')}`);

const bundle = buildBundle(story, { cta: true });
const checks = [
  ['HTML 含 doctype', bundle.html.startsWith('<!DOCTYPE html>')],
  ['HTML 含全部场景', (bundle.html.match(/class="scene"/g) || []).length === story.scenes.length],
  ['CSS 含逐幕关键帧', bundle.css.includes('@keyframes sc0') && bundle.css.includes(`@keyframes sc${story.scenes.length - 1}`)],
  ['CSS 无渐变', !/linear-gradient|radial-gradient/.test(bundle.css)],
  ['JSON 五层架构', storyToJSON(story).architecture.length === 5],
];
checks.forEach(([name, ok]) => console.log(`      ${ok ? '✓' : '✗'} ${name}`));
if (checks.some(([, ok]) => !ok)) process.exitCode = 1;

const font = resolveFont();
const record = {
  id: 'WH-SELFTEST-0001',
  createdAt: new Date().toISOString(),
  expiresAt: new Date(Date.now() + 90 * 864e5).toISOString(),
  name: '测试访客',
  email: 'test@example.com',
  storyTitle: story.title,
  scenes: story.stats.scenes,
  chars: story.stats.chars,
  castNames: story.cast.map((c) => c.name),
  fingerprint: story.fingerprint,
  source: 'selftest',
};

const pdf = await certificateBuffer(record, story);
if (!existsSync(join(root, 'logs'))) mkdirSync(join(root, 'logs'), { recursive: true });
const out = join(root, 'logs', 'lab-cert-selftest.pdf');
writeFileSync(out, pdf);
console.log(`[2/3] 证书：${(pdf.length / 1024).toFixed(1)} KB · 字体 ${font.label}${font.cjk ? '（中文已嵌入）' : '（未找到中文字体，降级英文）'}`);
console.log(`[3/3] 预览文件：${out}`);
console.log(pdf.length > 8000 ? '结果：通过' : '结果：PDF 体积异常，请检查');
if (!font.cjk) console.log('提示：安装 fonts-wqy-microhei 或把 TTF 放入 assets/fonts/ 即可输出中文证书');
