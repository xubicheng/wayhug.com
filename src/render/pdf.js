/**
 * PDF 渲染模块 (Delivery Layer - PDF)
 * 将生成的台词/文章导出为 PDF
 */

import PDFDocument from 'pdfkit';
import { findById } from '../data/store.js';
import { getCharacter } from '../engine/character.js';

/**
 * 生成内容的 PDF 并写入响应流
 * @param {string} contentId - 内容 ID
 * @param {Object} res - Express response
 */
export function streamContentPDF(contentId, res) {
  const content = findById('contents', contentId);
  if (!content) {
    res.status(404).json({ code: 404, message: '内容不存在' });
    return;
  }

  const doc = new PDFDocument({
    size: 'A4',
    margin: 50,
    info: {
      Title: content.title || content.topic || 'Wayhug AI 生成内容',
      Author: content.character?.name || 'Wayhug AI Engine',
      Producer: 'Wayhug AI Engine',
      Creator: 'Wayhug AI Engine'
    }
  });

  res.setHeader('Content-Type', 'application/pdf');
  res.setHeader('Content-Disposition', `inline; filename="wayhug-${contentId}.pdf"`);

  doc.pipe(res);

  // 标题
  doc.fontSize(22).fillColor('#2c3e50').text(content.title || content.topic || '无标题', {
    align: 'center'
  });

  doc.moveDown(0.5);

  // 元信息
  const charName = content.character?.name || '未知角色';
  const charEmoji = content.character?.emoji || '🐾';
  const moodName = content.moodName || '';
  doc.fontSize(11).fillColor('#7f8c8d').text(
    `${charEmoji} ${charName}    |    主题：${content.topic || '-'}    |    情绪：${moodName}`,
    { align: 'center' }
  );

  doc.moveDown(2);

  // 正文
  doc.fontSize(14).fillColor('#2c3e50');

  if (content.type === 'article' && Array.isArray(content.paragraphs)) {
    content.paragraphs.forEach((p, i) => {
      doc.text(p, { indent: 28, lineGap: 8 });
      doc.moveDown(1);
    });
  } else {
    doc.text(content.text || '', { indent: 28, lineGap: 8, align: 'justify' });
  }

  // 页脚
  doc.moveDown(3);
  doc.fontSize(9).fillColor('#bdc3c7').text(
    `由 Wayhug AI Engine 生成 · ${new Date(content.createdAt || Date.now()).toLocaleString('zh-CN')}`,
    { align: 'center' }
  );

  doc.end();
}

/**
 * 从内容对象直接生成 PDF Buffer
 */
export function contentToPDFBuffer(content) {
  return new Promise((resolve, reject) => {
    const doc = new PDFDocument({ size: 'A4', margin: 50 });
    const chunks = [];

    doc.on('data', chunk => chunks.push(chunk));
    doc.on('end', () => resolve(Buffer.concat(chunks)));
    doc.on('error', reject);

    doc.fontSize(18).text(content.title || content.topic || '无标题');
    doc.moveDown();

    if (Array.isArray(content.paragraphs)) {
      content.paragraphs.forEach(p => doc.text(p, { indent: 28 }));
    } else {
      doc.text(content.text || '', { indent: 28 });
    }

    doc.end();
  });
}
