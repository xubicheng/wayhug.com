/**
 * 内核层 · 中文字体解析
 * PDF 证书必须嵌入中文字体，否则 PDFKit 默认 Helvetica 会把中文画成空白/乱码。
 * 说明：.ttc 字体集合必须用 PostScript 名选择具体字重（如 MicrosoftYaHei），
 * 传中文家族名会让 PDFKit 直接报 “Not a supported font format”。
 */

import { existsSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import fontkit from 'fontkit';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..', '..');

let cached;

function scanRepoFonts() {
  const dir = join(root, 'assets', 'fonts');
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((f) => /\.(ttf|ttc|otf)$/i.test(f))
    .map((f) => ({ path: join(dir, f), family: undefined }));
}

function candidates() {
  return [
    { path: process.env.LAB_FONT_PATH || '', family: process.env.LAB_FONT_FAMILY || undefined },
    ...scanRepoFonts(),
    { path: 'C:/Windows/Fonts/msyh.ttc', family: 'MicrosoftYaHei' },
    { path: 'C:/Windows/Fonts/simhei.ttf' },
    { path: 'C:/Windows/Fonts/Deng.ttf' },
    { path: '/usr/share/fonts/truetype/wqy/wqy-microhei.ttc', family: 'WenQuanYiMicroHei' },
    { path: '/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc', family: 'WenQuanYiZenHei' },
    { path: '/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc', family: 'NotoSansCJKsc-Regular' },
    { path: '/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc', family: 'NotoSansCJKsc-Regular' },
    { path: '/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf' },
  ];
}

/** 用 fontkit 预校验：能打开、且确实是单一字体（而非字体集合） */
function probe(path, family) {
  if (!path || !existsSync(path)) return false;
  try {
    const font = family ? fontkit.openSync(path, family) : fontkit.openSync(path);
    return Boolean(font) && typeof font.unitsPerEm === 'number';
  } catch {
    return false;
  }
}

export function resolveFont() {
  if (cached) return cached;
  for (const c of candidates()) {
    if (probe(c.path, c.family)) {
      cached = { cjk: true, path: c.path, family: c.family, label: c.path.split(/[\\/]/).pop() };
      return cached;
    }
  }
  cached = { cjk: false, path: null, family: null, label: 'Helvetica（未找到中文字体）' };
  return cached;
}
