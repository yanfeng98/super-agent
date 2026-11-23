import assert from 'node:assert/strict';
import test from 'node:test';
import { readExternalHistory, readExternalMessage } from '../../../src/channels/feishu/history-reader.mjs';

const identity = { botId: 'bot', appId: 'app', botOpenId: 'bot-open-id', fingerprint: 'a'.repeat(64) };
const route = { messageId: 'anchor', conversationId: 'conversation', actorId: 'actor' };
const signal = new AbortController().signal;

/** One message as Feishu returns it, addressed so the retained route keeps resolving to `anchor`. */
const record = (overrides = {}) => ({
  message_id: 'anchor', chat_id: 'conversation', msg_type: 'text',
  body: { content: JSON.stringify({ text: 'hello' }) },
  sender: { id: 'actor', sender_type: 'user', id_type: 'open_id' }, create_time: '1790787600000', ...overrides,
});
/** A client whose message read returns the record whose id it was asked for. */
const client = (...items) => ({
  im: { v1: { message: {
    get: async ({ path }) => ({ data: { items: items.filter(item => item.message_id === path.message_id) } }),
  } } },
});

test('reads another message of the conversation under the retained route', async () => {
  const reply = record({ message_id: 'reply', sender: { id: 'ou_bob', sender_type: 'user', id_type: 'open_id' },
    body: { content: JSON.stringify({ text: 'done' }) } });
  const read = await readExternalMessage(client(record(), reply), identity, route, signal, 'reply');
  assert.equal(read.messageId, 'reply');
  assert.equal(read.text, 'done');
  assert.equal(read.chatId, 'conversation');
  assert.equal(read.sender.id, 'ou_bob');
  assert.equal(read.deleted, false);
});

test('reports a recalled message apart from one it cannot render', async () => {
  await assert.rejects(
    readExternalMessage(client(record(), record({ message_id: 'gone', deleted: true })), identity, route, signal, 'gone'),
    { code: 'message-deleted' });
  await assert.rejects(
    readExternalMessage(client(record(), record({ message_id: 'system', msg_type: 'system' })), identity, route, signal, 'system'),
    { code: 'message-unavailable' });
  // Without a target id the route's own message is read directly; a consumer naming it by id gets `stale-route` instead,
  // because a recalled message never satisfies the route check.
  await assert.rejects(readExternalMessage(client(record({ deleted: true })), identity, route, signal),
    { code: 'message-deleted' });
  await assert.rejects(readExternalMessage(client(record({ deleted: true })), identity, route, signal, 'anchor'),
    { code: 'stale-route' });
});

test('refuses a route that no longer resolves to its own message', async () => {
  await assert.rejects(
    readExternalMessage(client(record({ chat_id: 'elsewhere' }), record({ message_id: 'reply' })), identity, route, signal, 'reply'),
    { code: 'stale-route' });
  // A recalled message of another chat stays as opaque as any other refusal, so this cannot probe other chats.
  await assert.rejects(
    readExternalMessage(client(record(), record({ message_id: 'gone', chat_id: 'elsewhere', deleted: true })),
      identity, route, signal, 'gone'),
    { code: 'stale-route' });
  // An id the provider no longer returns is unreadable rather than deleted.
  await assert.rejects(readExternalMessage(client(record({ message_id: 'other' })), identity, route, signal),
    { code: 'message-unavailable' });
});

test('a context page is as large as a chat history page', async () => {
  const page = Array.from({ length: 50 }, (_, index) => record({ message_id: `m${index}` }));
  const readClient = { im: { v1: { message: {
    get: async ({ path }) => ({ data: { items: path.message_id === 'anchor' ? [record()] : [] } }),
    list: async () => ({ data: { items: page, has_more: false } }),
  } } } };
  const read = await readExternalHistory(readClient, identity, route, { scope: 'group', limit: 50 }, signal);
  assert.equal(read.events.length, 50);
  assert.equal(read.hasMore, false);
  // The page cap is Feishu's 50, refused before any request for one more.
  let listed = 0;
  readClient.im.v1.message.list = async () => { listed++; return { data: { items: page, has_more: false } }; };
  await assert.rejects(readExternalHistory(readClient, identity, route, { scope: 'group', limit: 51 }, signal),
    { code: 'bad-request' });
  assert.equal(listed, 0);
});
