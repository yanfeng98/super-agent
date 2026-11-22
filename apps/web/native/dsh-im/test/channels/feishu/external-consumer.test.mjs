import assert from 'node:assert/strict';
import test from 'node:test';
import { normalizeExternalText, ExclusiveInboundConsumers } from '../../../src/channels/feishu/external-consumer.mjs';

const fingerprint = 'a'.repeat(64);
function event() {
  return {
    event_id: 'event-1', app_id: 'app',
    sender: { sender_type: 'user', sender_id: { open_id: 'actor' } },
    message: { message_id: 'message', chat_id: 'conversation', chat_type: 'group',
      message_type: 'text', content: JSON.stringify({ text: '@_user_1 hello' }),
      mentions: [{ id: { open_id: 'bot-open-id' }, key: '@_user_1', name: 'Platform Bot' }],
      create_time: '1790787600000', thread_id: 'thread', root_id: 'root', parent_id: 'parent' },
  };
}
function normalized(value = event()) {
  return normalizeExternalText(value, { botId: 'bot', appId: 'app', botOpenId: 'bot-open-id', fingerprint });
}

test('normalized authenticated text retains causal identities without raw payload or secrets', () => {
  const input = event();
  input.secret = 'private-material';
  const evidence = normalized(input);
  assert.deepEqual(evidence.reply, { messageId: 'message', conversationId: 'conversation', actorId: 'actor',
    threadId: 'thread', rootId: 'root', parentId: 'parent' });
  assert.equal(evidence.replay.gapPossible, true);
  assert.equal(Object.isFrozen(evidence.reply), true);
  assert.equal(JSON.stringify(evidence).includes('private-material'), false);
  input.message.chat_id = 'other';
  assert.equal(evidence.conversation.id, 'conversation');
  assert.throws(() => normalized({ ...input, app_id: 'other' }), { code: 'account-changed' });
  assert.equal(normalized({ ...input, sender: { sender_type: 'bot' } }), null);
  assert.equal(evidence.conversation.kind, 'group');
  assert.equal(evidence.mentionedAccount, true);
  assert.deepEqual(evidence.mentions, [{ id: 'bot-open-id', key: '@_user_1', name: 'Platform Bot' }]);
  assert.equal(Object.isFrozen(evidence.mentions[0]), true);
  assert.equal(normalized({ ...input, message: { ...input.message, chat_type: 'unknown' } }), null);
});

test('renders resource, card and system bodies instead of dropping them', () => {
  const body = (messageType, content) => normalized({ ...event(),
    message: { ...event().message, message_type: messageType, content: JSON.stringify(content) } });
  assert.equal(body('file', { file_key: 'file_v3_1', file_name: 'report.pdf' }).text, '[文件] report.pdf file_v3_1');
  assert.equal(body('audio', { file_key: 'file_v3_2', duration: 3200 }).text, '[语音] 3s file_v3_2');
  assert.equal(body('media', { file_key: 'file_v3_3', file_name: 'demo.mp4' }).text, '[视频] demo.mp4 file_v3_3');
  assert.equal(body('sticker', { file_key: 'file_v3_4' }).text, '[表情] file_v3_4');
  assert.equal(body('interactive', { schema: '2.0', body: { elements: [{ tag: 'markdown', content: 'deploy ok' }] } }).text,
    '[卡片] deploy ok');
  assert.equal(body('merge_forward', {}).text, '[合并转发]');
  // A consumer that read the forward's sub-messages hands them back through the same body.
  assert.equal(body('merge_forward', { text: 'Alex: ship it\nBot: done' }).text, 'Alex: ship it\nBot: done');
  assert.equal(body('share_chat', { chat_id: 'oc_shared' }).text, '[群名片] oc_shared');
  assert.equal(body('share_user', { user_id: 'ou_shared' }).text, '[个人名片] ou_shared');
  assert.throws(() => body('file', {}), { code: 'invalid-inbound' });
});

test('keeps an over-long body instead of dropping it, and says it was cut', () => {
  const long = normalized({ ...event(), message: { ...event().message, content: JSON.stringify({ text: 'x'.repeat(20000) }) } });
  assert.equal(long.text, `${'x'.repeat(16000)}\n[内容过长，已截断]`);
  const post = normalized({ ...event(), message: { ...event().message, message_type: 'post',
    content: JSON.stringify({ zh_cn: { content: [[{ tag: 'text', text: 'y'.repeat(20000) }]] } }) } });
  assert.equal(post.text.endsWith('\n[内容过长，已截断]'), true);
  assert.equal(post.text.length, 16000 + '\n[内容过长，已截断]'.length);
});

test('cuts an over-long body without splitting a surrogate pair', () => {
  const emoji = normalized({ ...event(), message: { ...event().message,
    content: JSON.stringify({ text: `${'x'.repeat(15999)}😀 tail` }) } });
  assert.equal(emoji.text, `${'x'.repeat(15999)}\n[内容过长，已截断]`);
  assert.equal(emoji.text.isWellFormed(), true);
});

test('carries the provider AppLink only when the provider supplies one', () => {
  const link = 'https://applink.feishu.cn/client/message/open?message_id=message';
  const withLink = normalized({ ...event(), message: { ...event().message, message_app_link: link } });
  assert.equal(withLink.appLink, link);
  assert.equal(normalized(event()).appLink, undefined);
});

test('exclusive registration waits for durable acceptance and disposal never substitutes another consumer', async () => {
  const consumers = new ExclusiveInboundConsumers();
  let commit;
  const committed = new Promise(resolve => { commit = resolve; });
  let calls = 0;
  const dispose = consumers.register('bot', { fingerprint, onEvent: async () => {
    calls++; await committed; return { accepted: true };
  } });
  assert.throws(() => consumers.register('bot', { fingerprint, onEvent() {} }), { code: 'consumer-conflict' });
  let acknowledged = false;
  const pending = consumers.accept('bot', normalized()).then(() => { acknowledged = true; });
  await Promise.resolve();
  assert.equal(acknowledged, false);
  commit(); await pending;
  assert.equal(calls, 1);
  dispose(); dispose();
  await assert.rejects(consumers.accept('bot', normalized()), { code: 'consumer-unavailable' });
});

test('consumer loss during commit rejects acknowledgement even when a new registration appears', async () => {
  const consumers = new ExclusiveInboundConsumers();
  let commit;
  const committed = new Promise(resolve => { commit = resolve; });
  const dispose = consumers.register('bot', { fingerprint, onEvent: async () => {
    await committed; return { accepted: true };
  } });
  const pending = consumers.accept('bot', normalized());
  dispose();
  consumers.register('bot', { fingerprint, onEvent: async () => ({ accepted: true }) });
  commit();
  await assert.rejects(pending, { code: 'consumer-unavailable' });
  consumers.close();
});

test('fingerprint mismatch, uncommitted result and canceled registrations fail closed', async () => {
  const consumers = new ExclusiveInboundConsumers();
  const controller = new AbortController();
  let calls = 0;
  consumers.register('bot', { fingerprint, signal: controller.signal, onEvent: async () => {
    calls++; return { accepted: false };
  } });
  await assert.rejects(consumers.accept('bot', { ...normalized(), fingerprint: 'b'.repeat(64) }), { code: 'account-changed' });
  assert.equal(calls, 0);
  await assert.rejects(consumers.accept('bot', normalized()), { code: 'ingress-not-accepted' });
  controller.abort();
  await assert.rejects(consumers.accept('bot', normalized()), { code: 'consumer-unavailable' });
});


test('ordinary group text and another Bot mention retain context without implying attention for this account', () => {
  const input = event();
  input.message.mentions = [{ id: { open_id: 'other-bot' }, key: '@_user_2' }];
  const otherMention = normalized(input);
  assert.equal(otherMention.mentionedAccount, false);
  assert.deepEqual(otherMention.mentions, [{ id: 'other-bot', key: '@_user_2' }]);
  input.message.mentions = [];
  input.message.content = JSON.stringify({ text: 'ordinary team context' });
  assert.equal(normalized(input).mentionedAccount, false);
  assert.equal(normalized(input).text, 'ordinary team context');
  input.message.chat_type = 'p2p';
  assert.equal(normalized(input).conversation.kind, 'dm');
});

test('group evidence requires authenticated account identity and valid mention and timestamp fields', () => {
  assert.throws(() => normalizeExternalText(event(), { botId: 'bot', appId: 'app', fingerprint }), { code: 'account-unverified' });
  const input = event();
  input.message.mentions = [{ id: { open_id: 'bot-open-id' }, key: '' }];
  assert.throws(() => normalized(input), { code: 'invalid-inbound' });
  input.message.mentions = [];
  input.message.create_time = '9e99';
  assert.throws(() => normalized(input), { code: 'invalid-inbound' });
});

test('post bodies are rendered to text with their image keys, and bot senders are admitted only on request', () => {
  const post = event();
  post.sender.sender_type = 'app';
  post.message.message_type = 'post';
  post.message.content = JSON.stringify({ zh_cn: { title: 'Build', content: [
    [{ tag: 'text', text: 'see ' }, { tag: 'at', user_id: 'bot-open-id', user_name: 'Platform Bot' }],
    [{ tag: 'a', text: 'logs', href: 'https://example.test/logs' }, { tag: 'img', image_key: 'img_1' }],
  ] } });
  assert.equal(normalizeExternalText(post, { botId: 'bot', appId: 'app', botOpenId: 'bot-open-id', fingerprint }), null);
  const evidence = normalizeExternalText(post, { botId: 'bot', appId: 'app', botOpenId: 'bot-open-id', fingerprint },
    { includeBots: true });
  assert.equal(evidence.actor.kind, 'bot');
  assert.equal(evidence.messageType, 'post');
  assert.equal(evidence.text, 'Build\nsee @Platform Bot\nlogs https://example.test/logs[图片] img_1');
  assert.deepEqual(evidence.images, ['img_1']);
});

test('image messages become a placeholder carrying the image key', () => {
  const image = event();
  image.message.message_type = 'image';
  image.message.content = JSON.stringify({ image_key: 'img_abc' });
  const evidence = normalized(image);
  assert.equal(evidence.messageType, 'image');
  assert.equal(evidence.text, '[图片] img_abc');
  assert.deepEqual(evidence.images, ['img_abc']);
  assert.throws(() => normalized({ ...image, message: { ...image.message, content: '{}' } }), { code: 'invalid-inbound' });
  assert.throws(() => normalized({ ...image, message: { ...image.message, content: 'not-json' } }), { code: 'invalid-inbound' });
});

test('a bot sender identified only by app_id is admitted on request and keeps that id', () => {
  const bot = event();
  bot.sender = { sender_type: 'app', sender_id: { app_id: 'cli_ci' } };
  bot.message.mentions = [];
  assert.equal(normalizeExternalText(bot, { botId: 'bot', appId: 'app', botOpenId: 'bot-open-id', fingerprint }), null);
  const evidence = normalizeExternalText(bot, { botId: 'bot', appId: 'app', botOpenId: 'bot-open-id', fingerprint },
    { includeBots: true });
  assert.equal(evidence.actor.kind, 'bot');
  assert.equal(evidence.actor.id, 'cli_ci');
  assert.equal(evidence.mentionedAccount, false);
});

test('a post that @s the bot counts as addressed even without a top-level mention list', () => {
  const post = event();
  post.message.mentions = [];
  post.message.message_type = 'post';
  post.message.content = JSON.stringify({ zh_cn: { content: [[
    { tag: 'at', user_id: 'bot-open-id', user_name: 'Platform Bot' }, { tag: 'text', text: ' build failed' },
  ]] } });
  const evidence = normalized(post);
  assert.equal(evidence.mentionedAccount, true);
  assert.equal(evidence.text, '@Platform Bot build failed');
});
