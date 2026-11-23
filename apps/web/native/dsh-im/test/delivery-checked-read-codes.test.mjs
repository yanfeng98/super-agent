import assert from 'node:assert/strict';
import test from 'node:test';
import { createDeliveryService } from '../plugin-src/host/delivery-service.mjs';

const fingerprint = 'a'.repeat(64);
const route = { messageId: 'om_1', conversationId: 'oc_team', actorId: 'ou_alice' };

/** A delivery adapter whose single-message read refuses with `code`. */
function adapter(code) {
  return {
    channel: 'feishu',
    ownsBot: async botId => botId === 'bot_one',
    listBots: async () => [],
    listTargets: async () => [],
    listSuggestions: async () => [],
    createTarget: async () => { throw new Error('unused'); },
    updateTarget: async () => { throw new Error('unused'); },
    deleteTarget: async () => { throw new Error('unused'); },
    sendText: async () => { throw new Error('unused'); },
    messageChecked: async () => { throw Object.assign(new Error(code), { code }); },
  };
}

/** Runs one refusing single-message read through the service, returning the code the caller receives. */
async function refusal(code) {
  const service = createDeliveryService();
  const dispose = service.registerAdapter(adapter(code));
  try {
    await service.messageChecked('bot_one', route, { expectedFingerprint: fingerprint });
    assert.fail('the read must refuse');
  } catch (error) {
    return error.code;
  } finally {
    dispose();
  }
}

test('a single-message read keeps the reason it has no content', async () => {
  // The caller reports these apart from an empty conversation, so the delivery layer must not flatten them.
  assert.equal(await refusal('message-deleted'), 'message-deleted');
  assert.equal(await refusal('message-unavailable'), 'message-unavailable');
  assert.equal(await refusal('stale-route'), 'stale-route');
  // An internal failure stays opaque instead of naming something the caller could not handle.
  assert.equal(await refusal('internal-detail'), 'history-unavailable');
});

test('a history read forwards the context options it was given', async () => {
  // The Source asks a topic read for the bot's own messages and a recalled message's place; the delivery layer must not
  // drop either option on its way to the adapter.
  const seen = [];
  const service = createDeliveryService();
  const dispose = service.registerAdapter({
    ...adapter('unused'),
    historyChecked: async (_botId, _route, query) => { seen.push(query); return { version: 1 }; },
    threadHistoryChecked: async (_botId, query) => { seen.push(query); return { version: 1 }; },
  });
  try {
    await service.historyChecked('bot_one', route,
      { scope: 'nearby', limit: 20, includeBots: true, includeDeleted: true }, { expectedFingerprint: fingerprint });
    await service.threadHistoryChecked('bot_one',
      { chatId: 'oc_team', threadId: 'omt_topic', limit: 20, includeBots: true, includeSelf: true, includeDeleted: true },
      { expectedFingerprint: fingerprint });
  } finally {
    dispose();
  }
  assert.deepEqual(seen, [
    { scope: 'nearby', limit: 20, includeBots: true, includeDeleted: true },
    { chatId: 'oc_team', threadId: 'omt_topic', limit: 20, includeBots: true, includeSelf: true, includeDeleted: true },
  ]);
});
