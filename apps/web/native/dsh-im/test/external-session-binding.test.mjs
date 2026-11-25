import assert from 'node:assert/strict';
import test from 'node:test';

import { createDeliveryService } from '../plugin-src/host/delivery-service.mjs';

test('an externally bound Session is described by the conversation its owner supplied', async () => {
  const delivery = createDeliveryService();
  assert.equal(await delivery.describeConversation('orchestrator'), null);

  delivery.bindExternalSessionConversation('orchestrator', {
    channel: 'feishu', botId: 'bot-1', conversationKey: 'group:chat-1', chatId: 'chat-1',
  });

  assert.deepEqual(await delivery.describeConversation('orchestrator'), {
    channel: 'feishu', botId: 'bot-1', conversationKey: 'group:chat-1', chatId: 'chat-1', botName: null,
  });
  assert.equal(delivery.hasExternalSessionConversation('orchestrator'), true);
  assert.deepEqual(delivery.describeConversationSync('orchestrator'), {
    channel: 'feishu', botId: 'bot-1', conversationKey: 'group:chat-1', chatId: 'chat-1', botName: null,
  });

  delivery.clearExternalSessionConversation('orchestrator');
  assert.equal(delivery.hasExternalSessionConversation('orchestrator'), false);
  assert.equal(await delivery.describeConversation('orchestrator'), null);
});

test('an invalid external binding is rejected without changing the existing one', async () => {
  const delivery = createDeliveryService();
  delivery.bindExternalSessionConversation('orchestrator', {
    channel: 'feishu', botId: 'bot-1', conversationKey: 'group:chat-1',
  });

  assert.throws(() => delivery.bindExternalSessionConversation('orchestrator', {
    channel: 'feishu', botId: 'bad bot id', conversationKey: 'group:chat-2',
  }), (error) => error.code === 'bad-request');
  assert.equal((await delivery.describeConversation('orchestrator')).conversationKey, 'group:chat-1');
});

test('answer suppression is consumed once and scoped to one Session', () => {
  const delivery = createDeliveryService();
  assert.equal(delivery.consumeSessionSyncAnswerSuppression('orchestrator'), false);
  delivery.suppressNextSessionSyncAnswer('orchestrator');
  assert.equal(delivery.consumeSessionSyncAnswerSuppression('worker'), false);
  assert.equal(delivery.consumeSessionSyncAnswerSuppression('orchestrator'), true);
  assert.equal(delivery.consumeSessionSyncAnswerSuppression('orchestrator'), false);
});
