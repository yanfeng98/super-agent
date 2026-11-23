/** Identifiers and text helpers shared by the dsh-im-backed Feishu Source and the modules that read its records. */
import { key } from '../core/index.ts'

export const SOURCE = 'feishu'
/** Deterministic Trigger id of one Feishu message; every read path of the handoff produces the same id. */
export const feishuTriggerId = (messageId: string): string => key(SOURCE, messageId)
/** Writes each mention's name in place of the placeholder Feishu leaves in a message body. */
export const withMentionNames = (text: string, mentions: { key?: string | undefined; name?: string | undefined }[]): string =>
  mentions.reduce((result, mention) => mention.key && mention.name ? result.replaceAll(mention.key, `@${mention.name}`) : result, text)
/** Feishu or Lark AppLink that opens a chat, used when the message's own link did not arrive with the event. */
export const feishuChatLink = (brand: 'feishu' | 'lark', chatId: string): string =>
  `https://applink.${brand === 'lark' ? 'larksuite.com' : 'feishu.cn'}/client/chat/open?openChatId=${encodeURIComponent(chatId)}`
/** Extracts Feishu image keys from message text so keys remain paired with the originating message; arbitrary remote IDs are not accepted. */
export function admissionImageKeys(text: string): string[] {
  return [...new Set(text.match(/\bimg_[a-zA-Z0-9_-]+\b/g) ?? [])]
}
/** Extracts Feishu file, voice, and video keys from message text, which is where the handoff renders their placeholders. */
export function admissionFileKeys(text: string): string[] {
  return [...new Set(text.match(/\bfile_[a-zA-Z0-9_-]+\b/g) ?? [])]
}
