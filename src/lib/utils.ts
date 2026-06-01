import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"
import { DOUYIN_EMOJI_MAP } from "./emoji-map"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * 从文本中提取所有唯一的 Emoji (Unicode)
 */
export function extractEmojis(text: string): string[] {
  if (!text) return [];
  // 匹配 Unicode Emoji 的正则
  const regex = /\p{Extended_Pictographic}/gu;
  const matches = text.match(regex);
  if (!matches) return [];
  return Array.from(new Set(matches));
}

/**
 * 将文本中的 [标签] 替换为对应的 Emoji (Unicode)
 * 如果没有映射，则保持原样
 */
export function renderDouyinText(text: string): string {
  if (!text) return '';
  
  // 1. 先进行简单的 HTML 转义，防止 XSS
  const escaped = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');

  // 2. 匹配 [xxx] 格式的标签并替换
  return escaped.replace(/\[([^\]]{1,10})\]/g, (match) => {
    return DOUYIN_EMOJI_MAP[match] || match;
  });
}

/**
 * 从文本中提取抖音风格的标签 [表情]，并去重
 */
export function extractDouyinTags(text: string): string[] {
  if (!text) return [];
  const regex = /\[[^\]]{1,10}\]/g;
  const matches = text.match(regex);
  if (!matches) return [];
  return Array.from(new Set(matches));
}