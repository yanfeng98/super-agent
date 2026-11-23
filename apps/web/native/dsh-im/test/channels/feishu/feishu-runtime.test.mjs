import assert from 'node:assert/strict';
import test from 'node:test';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { FeishuRuntime } from '../../../src/channels/feishu/feishu-runtime.mjs';
import { FeishuHarnessBridge } from '../../../src/channels/feishu/bridge.mjs';
import { rememberConnectionTestTarget } from '../../../src/channels/shared/connection-test.mjs';
import { createSessionSyncCoordinator } from '../../../plugin-src/host/session-sync-coordinator.mjs';
import { DENY_ALL_ACCESS_POLICY, createAccessPolicy, createAccessPolicyScope } from '../../../src/channels/shared/access-policy.mjs';

class FakeClient {
  static instances = [];
  static sent = [];

  constructor(options) {
    this.options = options;
    this.im = {
      v1: {
        message: {
          create: async (payload) => {
            FakeClient.sent.push(payload);
            return { code: 0, data: { message_id: `message-${FakeClient.sent.length}` } };
          },
        },
      },
    };
    FakeClient.instances.push(this);
  }
}

class FakeDispatcher {
  register(handlers) {
    this.handlers = handlers;
    return this;
  }
}

class FakeWSClient {
  static instances = [];

  constructor(options) {
    this.options = options;
    this.state = 'idle';
    FakeWSClient.instances.push(this);
  }

  async start({ eventDispatcher } = {}) {
    this.state = 'connecting';
    this.dispatcher = eventDispatcher;
  }

  becomeReady() {
    this.state = 'connected';
    this.options.onReady();
  }

  fail(error = new Error('synthetic WebSocket failure')) {
    this.state = 'failed';
    this.options.onError(error);
  }

  beginReconnecting() {
    this.state = 'reconnecting';
    this.options.onReconnecting();
  }

  becomeReconnected() {
    this.state = 'connected';
    this.options.onReconnected();
  }

  becomeIdle() {
    this.state = 'idle';
  }

  getConnectionStatus() {
    return { state: this.state };
  }

  close() {
    this.state = 'closed';
  }
}

function fakeLark() {
  FakeWSClient.instances.length = 0;
  FakeClient.instances.length = 0;
  FakeClient.sent.length = 0;
  return {
    Domain: { Feishu: 'feishu-domain', Lark: 'lark-domain' },
    LoggerLevel: { info: 'info' },
    Client: FakeClient,
    EventDispatcher: FakeDispatcher,
    WSClient: FakeWSClient,
    defaultHttpInstance: {
      request: async (options) => options,
      get: async (_url, options) => options,
      delete: async (_url, options) => options,
      head: async (_url, options) => options,
      options: async (_url, options) => options,
      post: async (_url, _data, options) => options,
      put: async (_url, _data, options) => options,
      patch: async (_url, _data, options) => options,
    },
  };
}

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

async function waitFor(predicate, timeoutMs = 1_000) {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error('condition timed out');
    await new Promise((resolve) => setImmediate(resolve));
  }
}

test('FeishuRuntime becomes chat-ready only after Harness and Feishu are connected', async () => {
  let harnessChecks = 0;
  let harnessSignal;
  const wsAgent = { addRequest() {} };
  const runtime = new FeishuRuntime({
    lark: fakeLark(),
    appId: 'cli_test',
    appSecret: 'secret',
    wsAgent,
    ownerOpenIds: ['*', 'ou_owner'],
    harness: {
      async ensureRunning(options) {
        harnessChecks += 1;
        harnessSignal = options.signal;
      },
    },
    state: { hasSeen: () => false },
    connectTimeoutMs: 1_234,
  });

  assert.equal(runtime.status.ready, false);
  let settled = false;
  const starting = runtime.start().then((value) => {
    settled = true;
    return value;
  });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(settled, false);
  assert.equal(runtime.status.feishuLongConnectionState, 'connecting');
  assert.equal(FakeWSClient.instances[0].options.agent, wsAgent);
  assert.equal(FakeWSClient.instances[0].options.handshakeTimeoutMs, 1_234);
  assert.equal('agent' in FakeClient.instances[0].options, false);
  FakeWSClient.instances[0].becomeReady();
  const status = await starting;
  assert.equal(harnessChecks, 1);
  assert.equal(status.ready, true);
  assert.equal(status.feishuLongConnectionState, 'connected');
  assert.equal(status.harnessReachable, true);
  assert.equal(harnessSignal.aborted, false);
  assert.equal((await FakeClient.instances[0].options.httpInstance.request({
    url: 'https://open.feishu.cn/test',
  })).timeout, 15_000);

  const firstDispatcher = FakeWSClient.instances[0].dispatcher;
  FakeWSClient.instances[0].beginReconnecting();
  const reconnecting = await runtime.start();
  assert.equal(reconnecting.ready, false);
  assert.equal(reconnecting.feishuLongConnectionState, 'reconnecting');
  assert.equal(harnessChecks, 1);
  assert.equal(FakeWSClient.instances.length, 1);
  assert.equal(FakeClient.instances.length, 1);
  assert.equal(FakeWSClient.instances[0].dispatcher, firstDispatcher);
  FakeWSClient.instances[0].becomeReconnected();
  assert.equal(runtime.status.ready, true);
  assert.equal(runtime.status.feishuLongConnectionState, 'connected');

  assert.deepEqual(await runtime.sendConnectionTest('连接测试'), { sent: true });
  assert.deepEqual(FakeClient.sent, [{
    params: { receive_id_type: 'open_id' },
    data: {
      receive_id: 'ou_owner',
      msg_type: 'text',
      content: JSON.stringify({ text: '连接测试' }),
    },
  }]);

  assert.deepEqual(await runtime.sendProactiveText({
    kind: 'group',
    route: { chatId: 'oc_proactive_group' },
  }, '主动投递'), { sent: true });
  assert.deepEqual(FakeClient.sent[1], {
    params: { receive_id_type: 'chat_id' },
    data: {
      receive_id: 'oc_proactive_group',
      msg_type: 'text',
      content: JSON.stringify({ text: '主动投递' }),
    },
  });

  const stopped = await runtime.stop();
  assert.equal(stopped.ready, false);
  assert.equal(stopped.feishuLongConnectionState, 'idle');
  assert.equal(FakeWSClient.instances[0].state, 'closed');
  assert.equal(harnessSignal.aborted, true);

  const stoppedStatus = runtime.status;
  FakeWSClient.instances[0].becomeReady();
  assert.deepEqual(runtime.status, stoppedStatus);
  FakeWSClient.instances[0].fail(new Error('late failure after stop'));
  assert.deepEqual(runtime.status, stoppedStatus);
  FakeWSClient.instances[0].beginReconnecting();
  assert.deepEqual(runtime.status, stoppedStatus);
  FakeWSClient.instances[0].becomeReconnected();
  assert.deepEqual(runtime.status, stoppedStatus);
});

test('FeishuRuntime uses the selected domain for both HTTP and WebSocket clients', async () => {
  for (const domain of [undefined, 'feishu', 'lark']) {
    const runtime = new FeishuRuntime({
      lark: fakeLark(),
      appId: 'cli_domain',
      appSecret: 'secret',
      domain,
      ownerOpenIds: ['ou_owner'],
      harness: { async ensureRunning() {} },
      state: { hasSeen: () => false },
    });
    try {
      const starting = runtime.start();
      await waitFor(() => FakeWSClient.instances.length === 1);
      FakeWSClient.instances[0].becomeReady();
      await starting;
      const expected = domain === 'lark' ? 'lark-domain' : 'feishu-domain';
      assert.equal(FakeClient.instances[0].options.domain, expected);
      assert.equal(FakeWSClient.instances[0].options.domain, expected);
    } finally {
      await runtime.stop();
    }
  }
});

test('FeishuRuntime keeps Slash registration non-blocking and aborts it on stop', async () => {
  const lark = fakeLark();
  const requests = [];
  let createSignal;
  lark.defaultHttpInstance.request = async (options) => {
    requests.push(options);
    if (options.url.includes('/tenant_access_token/')) {
      return { code: 0, tenant_access_token: 'tenant-token' };
    }
    if (options.method === 'GET') return { code: 0, data: { items: [] } };
    createSignal = options.signal;
    return new Promise((_resolve, reject) => {
      const abort = () => reject(createSignal.reason);
      createSignal.addEventListener('abort', abort, { once: true });
      if (createSignal.aborted) abort();
    });
  };
  const runtime = new FeishuRuntime({
    lark,
    appId: 'cli_slash',
    appSecret: 'secret',
    ownerOpenIds: ['ou_owner'],
    harness: { async ensureRunning() {} },
    state: { hasSeen: () => false },
    logger: { info() {}, warn() {}, error() {} },
  });

  const starting = runtime.start();
  await new Promise((resolve) => setImmediate(resolve));
  FakeWSClient.instances[0].becomeReady();
  const ready = await starting;
  assert.equal(ready.ready, true);
  await waitFor(() => createSignal !== undefined);
  assert.equal(runtime.status.slashCommandRegistration, 'registering');
  assert.equal(requests.filter((request) => request.url.includes('/tenant_access_token/')).length, 1);

  await runtime.stop();
  assert.equal(createSignal.aborted, true);
  assert.equal(runtime.status.slashCommandRegistration, 'idle');
});

test('FeishuRuntime can disable Slash registration', async () => {
  const lark = fakeLark();
  let requests = 0;
  lark.defaultHttpInstance.request = async () => {
    requests += 1;
    return { code: 0 };
  };
  const runtime = new FeishuRuntime({
    lark,
    appId: 'cli_no_slash',
    appSecret: 'secret',
    ownerOpenIds: ['ou_owner'],
    harness: { async ensureRunning() {} },
    state: { hasSeen: () => false },
    slashCommands: false,
  });

  const starting = runtime.start();
  await new Promise((resolve) => setImmediate(resolve));
  FakeWSClient.instances[0].becomeReady();
  await starting;
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(requests, 0);
  assert.equal(runtime.status.slashCommandRegistration, 'idle');
  await runtime.stop();
});

test('FeishuRuntime stops a superseded panel sync and serialises the next one', async () => {
  // 上一轮同步还没跑完就再次保存：旧轮必须停在下一个检查点（不再按旧计划创建），
  // 新轮排在它之后跑，最终面板收敛到最新配置（review 反馈）。
  const lark = fakeLark();
  const remote = new Map();          // command -> command_id
  const created = [];
  const requestLog = [];
  let logAtSave = -1;
  let runtime;
  lark.defaultHttpInstance.request = async (options) => {
    const path = options.url.split('/open-apis/')[1] ?? options.url;
    requestLog.push(`${options.method} ${path}`);
    if (path === 'auth/v3/tenant_access_token/internal') {
      return { code: 0, tenant_access_token: 'tenant-token' };
    }
    if (options.method === 'GET' && path === 'application/v7/app_slash_commands') {
      return {
        code: 0,
        data: {
          items: [...remote.entries()].map(([command, command_id], index) => ({
            command, command_id, create_time: String(index + 1),
          })),
        },
      };
    }
    if (options.method === 'POST' && path === 'application/v7/app_slash_commands') {
      const command = options.data.command;
      remote.set(command, `id-${command}`);
      created.push(command);
      if (created.length === 1) {
        // 第一轮正在创建第一个指令时，用户保存了空面板。
        logAtSave = requestLog.length;
        runtime.setSlashPanel({ mode: 'custom', order: [] });
      }
      return { code: 0, data: { command_id: `id-${command}` } };
    }
    const match = /^application\/v7\/app_slash_commands\/(.+)$/.exec(path);
    if (match) {
      const entry = [...remote.entries()].find(([, id]) => id === match[1]);
      if (entry) remote.delete(entry[0]);
      return { code: 0, data: {} };
    }
    throw new Error(`unexpected ${options.method} ${options.url}`);
  };
  runtime = new FeishuRuntime({
    lark,
    appId: 'cli_panel_race',
    appSecret: 'secret',
    ownerOpenIds: ['ou_owner'],
    harness: { async ensureRunning() {} },
    state: { hasSeen: () => false },
    logger: { info() {}, warn() {}, error() {} },
  });
  // 启动前定好面板（此时还没有 http 实例，setSlashPanel 只记录配置）。
  runtime.setSlashPanel({ mode: 'custom', order: ['help', 'new'] });

  const starting = runtime.start();
  await new Promise((resolve) => setImmediate(resolve));
  FakeWSClient.instances[0].becomeReady();
  await starting;
  await waitFor(() => runtime.status.slashCommandRegistration === 'done');

  // 逆序重建：先建 new；被取代后不得再建 help。
  assert.deepEqual(created, ['new'], '被取代的那一轮不得继续按旧计划创建');
  // 新轮按最新配置（空面板）收敛：删掉刚建的 new。
  assert.equal(remote.size, 0, '面板必须收敛到最新配置');
  assert.deepEqual(requestLog.slice(logAtSave), [
    'POST auth/v3/tenant_access_token/internal',
    'GET application/v7/app_slash_commands',
    'DELETE application/v7/app_slash_commands/id-new',
  ], '新轮必须排在旧轮之后，旧轮不得再发请求');
  // 状态来自最新一轮，而不是被取代的那一轮。
  assert.equal(runtime.status.slashCommandsRegistered, 0);
  assert.equal(runtime.status.slashCommandsRemoved, 1);
  assert.equal(runtime.status.slashCommandsFailed, 0);
  await runtime.stop();
});

test('FeishuRuntime uses a remembered private target for wildcard-only manual bots', async () => {
  const state = { hasSeen: () => false };
  const runtime = new FeishuRuntime({
    lark: fakeLark(),
    appId: 'cli_manual',
    appSecret: 'secret',
    ownerOpenIds: ['*'],
    harness: { async ensureRunning() {} },
    state,
  });

  const starting = runtime.start();
  await new Promise((resolve) => setImmediate(resolve));
  FakeWSClient.instances[0].becomeReady();
  await starting;

  await assert.rejects(
    runtime.sendConnectionTest('连接测试'),
    (error) => error?.code === 'test-target-unavailable',
  );
  rememberConnectionTestTarget(state, { chatId: 'oc_manual_private' });
  assert.deepEqual(await runtime.sendConnectionTest('连接测试'), { sent: true });
  assert.deepEqual(FakeClient.sent, [{
    params: { receive_id_type: 'chat_id' },
    data: {
      receive_id: 'oc_manual_private',
      msg_type: 'text',
      content: JSON.stringify({ text: '连接测试' }),
    },
  }]);

  await runtime.stop();
});

test('FeishuRuntime stop waits for a pending Harness check and prevents startup resurrection', async () => {
  const harnessReady = deferred();
  let harnessSignal;
  const runtime = new FeishuRuntime({
    lark: fakeLark(),
    appId: 'cli_delayed_harness',
    appSecret: 'secret',
    ownerOpenId: 'ou_owner',
    harness: {
      async ensureRunning({ signal }) {
        harnessSignal = signal;
        await harnessReady.promise;
      },
    },
    state: { hasSeen: () => false },
  });

  const starting = runtime.start();
  const startRejected = assert.rejects(starting, (error) => error?.name === 'AbortError');
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(harnessSignal.aborted, false);

  let stopSettled = false;
  const stopping = runtime.stop().then((status) => {
    stopSettled = true;
    return status;
  });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(harnessSignal.aborted, true);
  assert.equal(stopSettled, false);
  assert.equal(FakeClient.instances.length, 0);
  assert.equal(FakeWSClient.instances.length, 0);

  harnessReady.resolve();
  await startRejected;
  const stopped = await stopping;
  assert.equal(stopped.ready, false);
  assert.equal(stopped.feishuLongConnectionState, 'idle');
  assert.equal(FakeClient.instances.length, 0);
  assert.equal(FakeWSClient.instances.length, 0);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(runtime.status, stopped);
});

test('FeishuRuntime bounds the bridge idle wait during stop', async t => {
  const warnings = [];
  const runtime = await startRuntimeForProbe({
    slashCommands: false,
    logger: { warn: (message) => warnings.push(message) },
  });
  const idle = deferred();
  t.after(() => idle.resolve());
  t.mock.method(FeishuHarnessBridge.prototype, 'waitForIdle', () => idle.promise);
  t.mock.timers.enable({ apis: ['setTimeout'] });

  let stopped;
  const stopping = runtime.stop().then((status) => { stopped = status; });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(runtime.status.ready, false);
  assert.equal(FakeWSClient.instances[0].state, 'closed');
  assert.equal(stopped, undefined);

  t.mock.timers.tick(1_999);
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(stopped, undefined);
  assert.deepEqual(warnings, []);

  t.mock.timers.tick(1);
  await new Promise((resolve) => setImmediate(resolve));
  assert.notEqual(stopped, undefined, 'stop must finish after the 2000ms bridge idle deadline');
  await stopping;
  assert.equal(stopped.ready, false);
  assert.equal(stopped.feishuLongConnectionState, 'idle');
  assert.equal(stopped.slashCommandRegistration, 'idle');
  assert.equal(stopped.error, null);
  assert.equal(stopped.lastError, null);
  assert.equal(warnings.length, 1);
  assert.match(warnings[0], /bridge idle wait timed out after 2000ms/);
});

test('FeishuRuntime unreferences and clears the idle deadline when the bridge is already idle', async t => {
  const runtime = await startRuntimeForProbe({ slashCommands: false });
  t.mock.method(FeishuHarnessBridge.prototype, 'waitForIdle', async () => {});
  const setTimer = globalThis.setTimeout;
  const clearTimer = globalThis.clearTimeout;
  let timer;
  t.after(() => clearTimer(timer));
  const setTimeoutMock = t.mock.method(globalThis, 'setTimeout', (...args) => {
    timer = setTimer(...args);
    t.mock.method(timer, 'unref');
    return timer;
  });
  const clearTimeoutMock = t.mock.method(globalThis, 'clearTimeout');

  const stopped = await runtime.stop();
  assert.equal(stopped.feishuLongConnectionState, 'idle');
  assert.equal(setTimeoutMock.mock.callCount(), 1);
  assert.equal(setTimeoutMock.mock.calls[0].arguments[1], 2_000);
  assert.equal(timer.unref.mock.callCount(), 1);
  assert.ok(clearTimeoutMock.mock.calls.some((call) => call.arguments[0] === timer));
});

test('FeishuRuntime fails closed when the initial WebSocket handshake times out', async () => {
  const runtime = new FeishuRuntime({
    lark: fakeLark(),
    appId: 'cli_test',
    appSecret: 'secret',
    ownerOpenId: 'ou_owner',
    harness: { async ensureRunning() {} },
    state: { hasSeen: () => false },
    connectTimeoutMs: 10,
  });

  await assert.rejects(runtime.start(), /handshake timed out/);
  assert.equal(runtime.status.ready, false);
  assert.equal(runtime.status.feishuLongConnectionState, 'failed');
  assert.equal(FakeWSClient.instances[0].state, 'closed');
  assert.equal(FakeWSClient.instances[0].options.handshakeTimeoutMs, 10);

  const failedStatus = runtime.status;
  FakeWSClient.instances[0].becomeReady();
  assert.deepEqual(runtime.status, failedStatus);
  FakeWSClient.instances[0].fail(new Error('late failure after timeout'));
  assert.deepEqual(runtime.status, failedStatus);
  FakeWSClient.instances[0].beginReconnecting();
  assert.deepEqual(runtime.status, failedStatus);
  FakeWSClient.instances[0].becomeReconnected();
  assert.deepEqual(runtime.status, failedStatus);
});

test('FeishuRuntime fails closed when Harness is unavailable', async () => {
  const runtime = new FeishuRuntime({
    lark: fakeLark(),
    appId: 'cli_test',
    appSecret: 'secret',
    ownerOpenId: 'ou_owner',
    harness: {
      async ensureRunning() { throw new Error('Harness unavailable'); },
    },
    state: { hasSeen: () => false },
  });

  await assert.rejects(runtime.start(), /Harness unavailable/);
  assert.equal(runtime.status.ready, false);
  assert.equal(runtime.status.feishuLongConnectionState, 'failed');
  assert.equal(runtime.status.error.details.stage, 'harness.check');
  assert.match(runtime.status.error.details.referenceId, /^IM-CONN-[A-F0-9]{8}$/);
  assert.doesNotMatch(JSON.stringify(runtime.status), /Harness unavailable/);
});

async function startRuntimeForProbe(options = {}) {
  const runtime = new FeishuRuntime({
    lark: fakeLark(),
    botId: 'bot_probe',
    appId: 'cli_probe',
    appSecret: 'secret',
    ownerOpenIds: ['ou_owner'],
    harness: { async ensureRunning() {} },
    state: { hasSeen: () => false },
    ...options,
  });
  const starting = runtime.start();
  await new Promise((resolve) => setImmediate(resolve));
  FakeWSClient.instances[0].becomeReady();
  await starting;
  return runtime;
}

for (const phase of ['user echo', 'final fallback']) {
  test(`a hanging proactive ${phase} times out so the next turn continues`, { timeout: 5_000 }, async t => {
    const runtime = await startRuntimeForProbe({ requestTimeoutMs: 15_000 });
    t.after(() => runtime.stop());
    t.mock.timers.enable({ apis: ['setTimeout'] });
    const pending = deferred();
    t.after(() => pending.resolve({ code: 0 }));
    const delivered = [];
    let held = false;
    const client = FakeClient.instances[0];
    const create = client.im.v1.message.create;
    client.im.v1.message.create = async request => {
      const text = JSON.parse(request.data.content).text;
      if (!held && text === (phase === 'user echo' ? 'question 1' : 'answer 1')) {
        held = true;
        return pending.promise;
      }
      return create(request);
    };
    const coordinator = createSessionSyncCoordinator({
      deliveryService: {
        // The Session binding resolves to this Feishu chat, exactly like the
        // production service: the coordinator itself knows no address.
        deliverSessionText: async (sessionId, text) => {
          delivered.push(text);
          return runtime.sendProactiveText({ kind: 'user', route: { openId: 'ou_owner' } }, text);
        },
      },
      // The user echo only happens for a mirrored Web input; keep it on so both
      // phases (echo, final fallback) are exercised.
      loadSettings: async () => ({ mirrorUserInput: true, deliverModelAnswers: true, inputIcon: '', modelIcon: '' }),
      logger: { warn() {} },
    });
    t.after(() => coordinator.close());
    const emitTurn = turn => {
      for (const event of [
        { type: 'turn/start', data: { turn } },
        {
          type: 'user/message',
          surfaceOp: 'append',
          data: {
            source: { kind: 'user', rpcId: `web-rpc-${turn}` },
            content: [{ type: 'text', text: `question ${turn}` }],
          },
        },
        { type: 'assistant/message', surfaceOp: 'append', data: { turn, step: 1, message: { content: [{ type: 'text', text: `answer ${turn}` }] } } },
        { type: 'turn/end', data: { turn, reason: { kind: 'completed' } } },
      ]) void coordinator.enqueue('session', event, 'dsh');
    };
    emitTurn(1);
    await waitFor(() => held);
    emitTurn(2);
    t.mock.timers.tick(15_000);
    await coordinator.whenIdle();
    // A timed-out input echo takes the answer of that same turn with it; the
    // next turn is unaffected.
    assert.equal(delivered.includes('answer 1'), phase === 'final fallback');
    assert.ok(delivered.includes('answer 2'));
    assert.ok(FakeClient.sent.some(request => JSON.parse(request.data.content).text === 'answer 2'));
    const sent = FakeClient.sent.length;
    pending.resolve({ code: 0 });
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(FakeClient.sent.length, sent, 'late acceptance must not retry or resume the old turn');
  });
}

for (const cause of ['caller', 'runtime']) {
  test(`a hanging proactive delivery releases on ${cause} cancellation`, { timeout: 5_000 }, async t => {
    const runtime = await startRuntimeForProbe();
    t.after(() => runtime.stop());
    const pending = deferred();
    t.after(() => pending.resolve({ code: 0 }));
    const entered = deferred();
    FakeClient.instances[0].im.v1.message.create = async () => {
      entered.resolve();
      return pending.promise;
    };
    const controller = new AbortController();
    const sending = runtime.sendProactiveText({ kind: 'user', route: { openId: 'ou_owner' } }, 'cancel me', {
      signal: controller.signal,
    });
    const rejected = assert.rejects(sending, { name: 'AbortError' });
    await entered.promise;
    if (cause === 'caller') controller.abort();
    else await runtime.stop();
    await rejected;
  });
}

test('FeishuRuntime drains failed and idle WS resources before creating a replacement', async () => {
  const runtime = await startRuntimeForProbe({
    logger: { info() {}, warn() {}, error() {} },
  });
  const firstWsClient = FakeWSClient.instances[0];
  firstWsClient.fail(new Error('terminal connection failure'));
  assert.equal(runtime.status.feishuLongConnectionState, 'failed');

  const restartingFromFailure = runtime.start();
  for (let attempt = 0; attempt < 20 && FakeWSClient.instances.length < 2; attempt += 1) {
    await new Promise((resolve) => setImmediate(resolve));
  }
  assert.equal(firstWsClient.state, 'closed');
  assert.equal(FakeWSClient.instances.length, 2);
  assert.equal(FakeClient.instances.length, 2);
  const secondWsClient = FakeWSClient.instances[1];
  secondWsClient.becomeReady();
  assert.equal((await restartingFromFailure).ready, true);

  // The SDK snapshot is authoritative even if a transition callback was
  // missed and Runtime status still says connected.
  secondWsClient.becomeIdle();
  assert.equal(runtime.status.ready, true);
  const restartingFromIdle = runtime.start();
  for (let attempt = 0; attempt < 20 && FakeWSClient.instances.length < 3; attempt += 1) {
    await new Promise((resolve) => setImmediate(resolve));
  }
  assert.equal(secondWsClient.state, 'closed');
  assert.equal(FakeWSClient.instances.length, 3);
  assert.equal(FakeClient.instances.length, 3);
  FakeWSClient.instances[2].becomeReady();
  assert.equal((await restartingFromIdle).ready, true);
  await runtime.stop();
});

function probeAction({ messageId = 'message-1', nonce, operatorOpenId = 'ou_owner' } = {}) {
  return {
    operator: { open_id: operatorOpenId },
    action: { value: { action: 'repair_verify', nonce } },
    context: { open_message_id: messageId },
  };
}

test('FeishuRuntime dispatcher ACKs immediately while card work is still pending', async () => {
  const seen = new Set();
  const runtime = await startRuntimeForProbe({
    logger: { info() {}, warn() {}, error() {} },
    harness: {
      async ensureRunning() {},
      async listWorkspaces() { return []; },
    },
    state: {
      hasSeen: (messageId) => seen.has(messageId),
      async markSeen(messageId) { seen.add(messageId); },
      sessionFor: () => null,
      includesArchivedSessions: () => false,
    },
  });
  const handlers = FakeWSClient.instances[0].dispatcher.handlers;

  assert.equal(handlers['im.message.reaction.created_v1']({}), undefined);
  assert.equal(handlers['im.message.reaction.deleted_v1']({}), undefined);
  assert.equal(handlers['im.message.receive_v1']({
    sender: {
      sender_type: 'user',
      sender_id: { open_id: 'ou_owner' },
    },
    message: {
      message_id: 'incoming-menu',
      chat_type: 'p2p',
      chat_id: 'oc_chat',
      message_type: 'text',
      content: JSON.stringify({ text: '/m' }),
    },
  }), undefined);

  for (let attempt = 0; attempt < 20 && FakeClient.sent.length === 0; attempt += 1) {
    await new Promise((resolve) => setImmediate(resolve));
  }
  assert.equal(FakeClient.sent[0]?.data?.msg_type, 'interactive');
  await new Promise((resolve) => setImmediate(resolve));

  let patchCalls = 0;
  let resolvePatch;
  const pendingPatch = new Promise((resolve) => { resolvePatch = resolve; });
  FakeClient.instances[0].im.v1.message.patch = async () => {
    patchCalls += 1;
    return pendingPatch;
  };

  const result = handlers['card.action.trigger']({
    operator: { open_id: 'ou_owner' },
    action: { value: { action: 'help' } },
    context: {
      open_message_id: 'message-1',
      open_chat_id: 'oc_chat',
    },
  });

  assert.equal(result, undefined);
  assert.equal(runtime.status.cardActionsReceived, 1);
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(patchCalls, 1);

  resolvePatch({ code: 0, data: { message_id: 'message-1' } });
  await pendingPatch;
  await new Promise((resolve) => setImmediate(resolve));
  await runtime.stop();
});

test('FeishuRuntime cancels a hanging card before restarting and ignores its late result', async t => {
  const seen = new Set();
  const runtime = await startRuntimeForProbe({
    logger: { info() {}, warn() {}, error() {} },
    harness: {
      async ensureRunning() {},
      async listWorkspaces() { return []; },
    },
    state: {
      hasSeen: (messageId) => seen.has(messageId),
      async markSeen(messageId) { seen.add(messageId); },
      sessionFor: () => null,
      includesArchivedSessions: () => false,
    },
  });
  const firstWsClient = FakeWSClient.instances[0];
  const handlers = firstWsClient.dispatcher.handlers;
  handlers['im.message.receive_v1']({
    sender: {
      sender_type: 'user',
      sender_id: { open_id: 'ou_owner' },
    },
    message: {
      message_id: 'restart-menu',
      chat_type: 'p2p',
      chat_id: 'oc_chat',
      message_type: 'text',
      content: JSON.stringify({ text: '/m' }),
    },
  });
  for (let attempt = 0; attempt < 20 && FakeClient.sent.length === 0; attempt += 1) {
    await new Promise((resolve) => setImmediate(resolve));
  }
  assert.equal(FakeClient.sent[0]?.data?.msg_type, 'interactive');
  await new Promise((resolve) => setImmediate(resolve));

  const patchEntered = deferred();
  const patchReleased = deferred();
  t.after(() => patchReleased.resolve({ code: 0 }));
  FakeClient.instances[0].im.v1.message.patch = async () => {
    patchEntered.resolve();
    return patchReleased.promise;
  };
  handlers['card.action.trigger']({
    operator: { open_id: 'ou_owner' },
    action: { value: { action: 'help' } },
    context: {
      open_message_id: 'message-1',
      open_chat_id: 'oc_chat',
    },
  });
  await patchEntered.promise;

  let stopSettled = false;
  const stopping = runtime.stop().then((status) => {
    stopSettled = true;
    return status;
  });
  let restartSettled = false;
  const restarting = runtime.start().then((status) => {
    restartSettled = true;
    return status;
  });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(stopSettled, true);
  assert.equal(restartSettled, false);
  assert.equal(firstWsClient.state, 'closed');

  const stopped = await stopping;
  assert.equal(stopped.ready, false);
  assert.equal(stopped.feishuLongConnectionState, 'idle');
  for (let attempt = 0; attempt < 20 && FakeWSClient.instances.length < 2; attempt += 1) {
    await new Promise((resolve) => setImmediate(resolve));
  }
  assert.equal(FakeWSClient.instances.length, 2);
  assert.equal(FakeClient.instances.length, 2);

  const secondWsClient = FakeWSClient.instances[1];
  secondWsClient.becomeReady();
  const restarted = await restarting;
  assert.equal(restarted.ready, true);
  assert.equal(restarted.feishuLongConnectionState, 'connected');
  assert.equal(secondWsClient.state, 'connected');
  patchReleased.resolve({ code: 0, data: { message_id: 'message-1' } });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(runtime.status.ready, true);
  assert.equal(secondWsClient.state, 'connected');

  const sentBeforeLateDispatch = FakeClient.sent.length;
  const cardActionsBeforeLateDispatch = runtime.status.cardActionsReceived;
  handlers['im.message.receive_v1']({
    sender: {
      sender_type: 'user',
      sender_id: { open_id: 'ou_owner' },
    },
    message: {
      message_id: 'late-old-dispatcher-message',
      chat_type: 'p2p',
      chat_id: 'oc_chat',
      message_type: 'text',
      content: JSON.stringify({ text: '/m' }),
    },
  });
  handlers['card.action.trigger']({
    operator: { open_id: 'ou_owner' },
    action: { value: { action: 'help' } },
    context: {
      open_message_id: 'message-1',
      open_chat_id: 'oc_chat',
    },
  });
  await new Promise((resolve) => setImmediate(resolve));
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(seen.has('late-old-dispatcher-message'), false);
  assert.equal(runtime.status.cardActionsReceived, cardActionsBeforeLateDispatch);
  assert.equal(FakeClient.sent.length, sentBeforeLateDispatch);

  assert.deepEqual(await runtime.sendConnectionTest('重启后连接测试'), { sent: true });
  await runtime.stop();
});

test('FeishuRuntime resolves a card-action probe only for the exact message, nonce and operator', async () => {
  const runtime = await startRuntimeForProbe();
  let settled = false;
  const probe = runtime.beginCardActionProbe({
    expectedOperatorOpenId: 'ou_owner',
    timeoutMs: 1_000,
  }).then((value) => {
    settled = true;
    return value;
  });
  await new Promise((resolve) => setImmediate(resolve));

  const request = FakeClient.sent[0];
  assert.deepEqual(request.params, { receive_id_type: 'open_id' });
  assert.equal(request.data.receive_id, 'ou_owner');
  assert.equal(request.data.msg_type, 'interactive');
  const card = JSON.parse(request.data.content);
  const behavior = card.body.elements[1].columns[0].elements[0].behaviors[0];
  assert.equal(behavior.value.action, 'repair_verify');
  const nonce = behavior.value.nonce;
  assert.match(nonce, /^[A-Za-z0-9_-]{16,128}$/);

  const dispatch = FakeWSClient.instances[0].dispatcher.handlers['card.action.trigger'];
  dispatch(probeAction({ messageId: 'message-other', nonce }));
  dispatch(probeAction({ nonce: `${nonce}x` }));
  dispatch(probeAction({ nonce, operatorOpenId: 'ou_other' }));
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(settled, false);

  dispatch(probeAction({ nonce }));
  assert.deepEqual(await probe, {
    verified: true,
    messageId: 'message-1',
    operatorOpenId: 'ou_owner',
  });
  assert.equal(runtime.status.cardActionsReceived, 4);
  assert.equal(runtime.status.cardActionProbesVerified, 1);
  assert.equal(FakeClient.sent.length, 2);
  assert.deepEqual(FakeClient.sent[1], {
    params: { receive_id_type: 'open_id' },
    data: {
      receive_id: 'ou_owner',
      msg_type: 'text',
      content: JSON.stringify({
        text: '✅ 修复完成：已实测收到 card.action.trigger，菜单按钮现在可用。',
      }),
    },
  });
  await runtime.stop();
});

test('FeishuRuntime times out and aborts pending card-action probes with stable codes', async () => {
  const runtime = await startRuntimeForProbe();
  await assert.rejects(
    runtime.beginCardActionProbe({ expectedOperatorOpenId: 'ou_owner', timeoutMs: 10 }),
    (error) => error?.code === 'card_action_probe_timeout',
  );
  await new Promise((resolve) => setImmediate(resolve));
  assert.match(
    JSON.parse(FakeClient.sent.at(-1).data.content).text,
    /修复验证超时.*不能确认按钮已修复.*不要重复授权/,
  );

  const pending = runtime.beginCardActionProbe({
    expectedOperatorOpenId: 'ou_owner',
    timeoutMs: 1_000,
  });
  await new Promise((resolve) => setImmediate(resolve));
  await runtime.stop();
  await assert.rejects(pending, (error) => error?.code === 'abort');
  await new Promise((resolve) => setImmediate(resolve));
  assert.match(
    JSON.parse(FakeClient.sent.at(-1).data.content).text,
    /修复验证中断.*不能确认修复成功.*不要重复授权/,
  );
});

test('FeishuRuntime reports probe-card send failure without masking its stable error', async () => {
  const runtime = await startRuntimeForProbe();
  const client = FakeClient.instances[0];
  client.im.v1.message.create = async (payload) => {
    FakeClient.sent.push(payload);
    if (payload.data.msg_type === 'interactive') return { code: 230001 };
    return { code: 0, data: { message_id: 'failure-notice' } };
  };

  await assert.rejects(
    runtime.beginCardActionProbe({ expectedOperatorOpenId: 'ou_owner', timeoutMs: 1_000 }),
    (error) => error?.code === 'card_action_probe_send_failed',
  );
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(FakeClient.sent.length, 2);
  assert.match(
    JSON.parse(FakeClient.sent[1].data.content).text,
    /修复验证失败.*不能确认 card\.action\.trigger 已恢复.*不要重复授权/,
  );
  await runtime.stop();
});

test('FeishuRuntime rejects imprecise probe operators and probes before connection', async () => {
  const runtime = new FeishuRuntime({
    lark: fakeLark(),
    botId: 'bot_probe',
    appId: 'cli_probe',
    appSecret: 'secret',
    ownerOpenIds: ['*'],
    harness: { async ensureRunning() {} },
    state: { hasSeen: () => false },
  });
  await assert.rejects(
    runtime.beginCardActionProbe({ expectedOperatorOpenId: 'ou_owner' }),
    (error) => error?.code === 'card_action_probe_unavailable',
  );

  const starting = runtime.start();
  await new Promise((resolve) => setImmediate(resolve));
  FakeWSClient.instances[0].becomeReady();
  await starting;
  await assert.rejects(
    runtime.beginCardActionProbe({ expectedOperatorOpenId: '*' }),
    /precise Feishu operator/,
  );
  await runtime.stop();
});


test('external group handler awaits durable acceptance and cannot start the native bridge or card actions', async () => {
  let calls = 0;
  const committed = deferred();
  const runtime = new FeishuRuntime({
    lark: fakeLark(), appId: 'app', appSecret: 'secret', ownerOpenIds: ['*'],
    consumerMode: 'external-consumer',
    acceptExternal: async () => { calls++; await committed.promise; return { accepted: true }; },
    harness: { async ensureRunning() {}, ask() { throw new Error('native bridge must not execute'); } },
    state: { hasSeen() { throw new Error('standalone state must not be consulted'); } },
  });
  const starting = runtime.start();
  await waitFor(() => FakeWSClient.instances.length === 1);
  const socket = FakeWSClient.instances[0];
  socket.becomeReady(); await starting;
  let acknowledged = false;
  const pending = socket.dispatcher.handlers['im.message.receive_v1']({ message: { chat_type: 'group', thread_id: 'thread' } }).then(() => { acknowledged = true; });
  await Promise.resolve();
  assert.equal(calls, 1); assert.equal(acknowledged, false);
  socket.dispatcher.handlers['card.action.trigger']({});
  assert.equal(runtime.status.cardActionsReceived, 0);
  committed.resolve(); await pending;
  assert.equal(acknowledged, true);
  await runtime.stop();
});

test('the bot access policy still gates a handed-over message, and the group archive still records it', async () => {
  const captured = [];
  const seen = [];
  const start = async (accessPolicy) => {
    const runtime = new FeishuRuntime({
      lark: fakeLark(), appId: 'app', appSecret: 'secret', ownerOpenIds: ['*'],
      consumerMode: 'external-consumer',
      acceptExternal: async (event) => { seen.push(event); return { accepted: true }; },
      accessPolicy,
      groupHistory: { capture: async (entry) => { captured.push(entry); return null; } },
      harness: { async ensureRunning() {} }, state: {}, logger: { info() {}, warn() {} },
    });
    const starting = runtime.start();
    await waitFor(() => FakeWSClient.instances.length === 1);
    FakeWSClient.instances[0].becomeReady(); await starting;
    return runtime;
  };
  const event = (id) => ({ message: { message_id: id, chat_id: 'chat', chat_type: 'group', message_type: 'text',
    content: JSON.stringify({ text: 'hello' }), create_time: '1790787600000' },
  sender: { sender_type: 'user', sender_id: { open_id: 'human' } } });
  // The owner's policy refuses this sender, so the consumer never sees the message; the group archive still records it.
  const denied = await start({ getSettings: () => DENY_ALL_ACCESS_POLICY });
  const result = await FakeWSClient.instances[0].dispatcher.handlers['im.message.receive_v1'](event('om_denied'));
  assert.deepEqual(result, { accepted: true, ignored: true });
  assert.equal(seen.length, 0);
  assert.equal(captured.length, 1);
  assert.equal(captured[0].groupId, 'chat');
  assert.equal(captured[0].text, 'hello');
  // A bot has no user id to match the whitelist against, so the consumer's own bot list decides its messages.
  const botEvent = event('om_bot');
  botEvent.sender = { sender_type: 'app', sender_id: { app_id: 'cli_ci' } };
  await FakeWSClient.instances[0].dispatcher.handlers['im.message.receive_v1'](botEvent);
  assert.equal(seen.length, 1);
  await denied.stop();
  // An open group policy admits the same message, so the consumer receives it.
  const open = createAccessPolicy({ group: createAccessPolicyScope({ mode: 'open',
    open: { defaultCanExecuteCommands: false, commandPermissionOverrides: [] }, allowlist: { users: [] } }) });
  const admitted = await start({ getSettings: () => open });
  await FakeWSClient.instances[0].dispatcher.handlers['im.message.receive_v1'](event('om_allowed'));
  assert.equal(seen.length, 2);
  assert.equal(captured.length, 3);
  await admitted.stop();
});

test('expands a merged forward for the external consumer and keeps the placeholder when it cannot be read', async () => {
  const seen = [];
  const runtime = new FeishuRuntime({
    lark: fakeLark(), appId: 'app', appSecret: 'secret', ownerOpenIds: ['*'],
    consumerMode: 'external-consumer',
    acceptExternal: async (event) => { seen.push(event); return { accepted: true }; },
    harness: { async ensureRunning() {} }, state: {},
  });
  const starting = runtime.start();
  await waitFor(() => FakeWSClient.instances.length === 1);
  FakeWSClient.instances[0].becomeReady(); await starting;
  const client = FakeClient.instances[0];
  const forward = { message_id: 'om-forward', chat_id: 'chat', chat_type: 'group', message_type: 'merge_forward',
    content: JSON.stringify({ content: 'Merged and Forwarded Message' }), create_time: '1790787600000' };
  const event = () => ({ message: { ...forward }, sender: { sender_type: 'user', sender_id: { open_id: 'human' } } });
  client.im.v1.message.get = async () => ({ code: 0, data: { items: [forward,
    { message_id: 'om-sub', upper_message_id: 'om-forward', msg_type: 'text',
      sender: { sender_name: 'Alex' }, body: { content: JSON.stringify({ text: 'ship it' }) } }] } });
  await FakeWSClient.instances[0].dispatcher.handlers['im.message.receive_v1'](event());
  assert.deepEqual(JSON.parse(seen[0].message.content), { text: 'Alex: ship it' });
  assert.equal(seen[0].message.message_type, 'merge_forward');
  // The event the SDK delivered is not mutated; a later read of it still sees the provider body.
  assert.equal(forward.content, JSON.stringify({ content: 'Merged and Forwarded Message' }));
  client.im.v1.message.get = async () => ({ code: 230027 });
  await FakeWSClient.instances[0].dispatcher.handlers['im.message.receive_v1'](event());
  assert.equal(seen[1].message.content, forward.content);
  await runtime.stop();
});

// 无 ffmpeg 环境:用可执行 sh 替身(见 fixtures/ffmpeg-stub.sh)。Windows 不能直接 spawn
// 脚本,这里只覆盖转写的时间点与结果,转写能力本身由 voice.test.mjs 覆盖。
const FFMPEG_STUB = fileURLToPath(new URL('./fixtures/ffmpeg-stub.sh', import.meta.url));
const VOICE_SPAWN_SKIP = process.platform === 'win32' ? 'requires the POSIX ffmpeg stub fixture' : false;

async function externalAudioRuntime(acceptExternal, { resource } = {}) {
  const runtime = new FeishuRuntime({
    lark: fakeLark(), appId: 'app', appSecret: 'secret', ownerOpenIds: ['*'],
    consumerMode: 'external-consumer',
    acceptExternal,
    voice: { config: { enabled: true, secretRef: 'DASHSCOPE_API_KEY', ffmpeg: FFMPEG_STUB }, secret: 'key' },
    harness: { async ensureRunning() {} }, state: {}, logger: { warn() {} },
  });
  const starting = runtime.start();
  await waitFor(() => FakeWSClient.instances.length === 1);
  FakeWSClient.instances[0].becomeReady(); await starting;
  FakeClient.instances[0].im.v1.messageResource = { get: resource };
  return runtime;
}

test('transcribes a voice message for the external consumer and keeps the provider body',
  { skip: VOICE_SPAWN_SKIP }, async () => {
    const seen = [];
    const runtime = await externalAudioRuntime(async (event) => { seen.push(event); return { accepted: true }; }, {
      resource: async () => ({ writeFile: async (path) => writeFileSync(path, Buffer.from('opus')) }),
    });
    const originalFetch = globalThis.fetch;
    globalThis.fetch = async () => new Response(JSON.stringify({ choices: [{ message: { content: '把导出按钮改成保存' } }] }),
      { status: 200, headers: { 'content-type': 'application/json' } });
    const content = JSON.stringify({ file_key: 'file_v3_voice', duration: 3200 });
    try {
      await FakeWSClient.instances[0].dispatcher.handlers['im.message.receive_v1']({
        message: { message_id: 'om-voice', chat_id: 'chat', chat_type: 'group', message_type: 'audio', content,
          create_time: '1790787600000' },
        sender: { sender_type: 'user', sender_id: { open_id: 'human' } },
      });
      assert.equal(seen[0].voiceTranscript, '把导出按钮改成保存');
      // The transcript rides beside the provider body instead of replacing it, so a consumer still sees the audio resource.
      assert.equal(seen[0].message.content, content);
      assert.equal(seen[0].message.message_type, 'audio');
    } finally {
      globalThis.fetch = originalFetch;
      await runtime.stop();
    }
  });

test('hands a voice message over unchanged when it cannot be transcribed', async () => {
  const seen = [];
  const runtime = await externalAudioRuntime(async (event) => { seen.push(event); return { accepted: true }; }, {
    resource: async () => { throw new Error('resource read failed'); },
  });
  const content = JSON.stringify({ file_key: 'file_v3_voice', duration: 3200 });
  const delivered = { message: { message_id: 'om-voice', chat_id: 'chat', chat_type: 'group', message_type: 'audio',
    content, create_time: '1790787600000' }, sender: { sender_type: 'user', sender_id: { open_id: 'human' } } };
  await FakeWSClient.instances[0].dispatcher.handlers['im.message.receive_v1'](delivered);
  assert.equal(seen[0].voiceTranscript, undefined);
  assert.equal(seen[0].message.content, content);
  await runtime.stop();
});

test('checked replies validate the original source before sending and retain unknown SDK outcomes', async () => {
  const runtime = new FeishuRuntime({ lark: fakeLark(), appId: 'app', appSecret: 'secret', ownerOpenIds: ['*'],
    consumerMode: 'external-consumer', harness: { async ensureRunning() {} }, state: {} });
  const starting = runtime.start();
  await waitFor(() => FakeWSClient.instances.length === 1);
  FakeWSClient.instances[0].becomeReady(); await starting;
  const source = { message_id: 'message', chat_id: 'chat', thread_id: 'thread', root_id: 'root', parent_id: 'parent', sender: { sender_type: 'user', id_type: 'open_id', id: 'human' } };
  const client = FakeClient.instances[0];
  client.im.v1.message.get = async () => ({ code: 0, data: { items: [source] } });
  let sends = 0;
  client.im.v1.message.reply = async request => {
    sends++;
    assert.equal(request.path.message_id, 'message');
    assert.equal(request.data.reply_in_thread, true);
    return { code: 0, data: { message_id: 'reply' } };
  };
  const route = { messageId: 'message', conversationId: 'chat', actorId: 'human', threadId: 'thread', rootId: 'root', parentId: 'parent' };
  assert.deepEqual(await runtime.replyChecked(route, 'hello'), { sent: true, messageId: 'reply' });
  source.thread_id = 'other-topic';
  await assert.rejects(runtime.replyChecked(route, 'hello'), { code: 'stale-route' });
  assert.equal(sends, 1);
  source.thread_id = 'thread';
  source.chat_id = 'other';
  await assert.rejects(runtime.replyChecked(route, 'hello'), { code: 'stale-route' });
  assert.equal(sends, 1);
  source.chat_id = 'chat';
  // Feishu opens a topic on a top-level message after the route was captured; the route still names that message.
  source.thread_id = 'omt_new'; source.root_id = undefined; source.parent_id = undefined;
  assert.deepEqual(await runtime.replyChecked({ messageId: 'message', conversationId: 'chat', actorId: 'human' }, 'hello',
    { replyInThread: true }), { sent: true, messageId: 'reply' });
  assert.equal(sends, 2);
  source.thread_id = 'thread'; source.root_id = 'root'; source.parent_id = 'parent';
  client.im.v1.message.reply = async () => { sends++; throw new Error('ambiguous timeout'); };
  await assert.rejects(runtime.replyChecked(route, 'hello'), { code: 'reply-result-unknown' });
  assert.equal(sends, 3);
  await runtime.stop();
});

test('a history read leaves out the messages this bot handled when the caller asks for fresh ones', async () => {
  const runtime = new FeishuRuntime({ lark: fakeLark(), appId: 'app', appSecret: 'secret', ownerOpenIds: ['*'],
    consumerMode: 'external-consumer', harness: { async ensureRunning() {} },
    // The state store remembers what the bot handled while it was not handed over.
    state: { hasSeen: id => id === 'om_handled' } });
  const starting = runtime.start();
  await waitFor(() => FakeWSClient.instances.length === 1);
  FakeWSClient.instances[0].becomeReady(); await starting;
  const items = ['om_handled', 'om_fresh'].map(id => ({ message_id: id, chat_id: 'chat', msg_type: 'text',
    body: { content: JSON.stringify({ text: id }) },
    sender: { id: 'human', sender_type: 'user', id_type: 'open_id' }, create_time: '1790830000000' }));
  FakeClient.instances[0].im.v1.message.list = async () => ({ code: 0, data: { items, has_more: false } });
  const identity = { botId: 'bot', appId: 'app', botOpenId: 'bot', fingerprint: 'a'.repeat(64) };
  const query = { chatId: 'chat', limit: 20, direct: true };
  assert.deepEqual((await runtime.chatHistoryChecked(identity, query)).events.map(event => event.messageId),
    ['om_handled', 'om_fresh']);
  assert.deepEqual((await runtime.chatHistoryChecked(identity, { ...query, excludeHandled: true })).events
    .map(event => event.messageId), ['om_fresh']);
  await runtime.stop();
});

test('checked history refuses a result after the runtime stops during SDK listing', async () => {
 const runtime=new FeishuRuntime({lark:fakeLark(),appId:'app',appSecret:'secret',ownerOpenIds:['*'],
 consumerMode:'external-consumer',harness:{async ensureRunning(){}},state:{}});
 const starting=runtime.start();await waitFor(()=>FakeWSClient.instances.length===1);
 FakeWSClient.instances[0].becomeReady();await starting;
 const client=FakeClient.instances[0];
 const source={message_id:'anchor',chat_id:'chat',create_time:'1790830000000',
 sender:{sender_type:'user',id_type:'open_id',id:'human'}};
 client.im.v1.message.get=async()=>({code:0,data:{items:[source]}});
 let entered,release;const started=new Promise(resolve=>{entered=resolve;});const gate=new Promise(resolve=>{release=resolve;});
 client.im.v1.message.list=async()=>{entered();await gate;return {code:0,data:{items:[],has_more:false}};};
 const pending=runtime.historyChecked({botId:'bot',appId:'app',botOpenId:'bot',fingerprint:'a'.repeat(64)},
 {messageId:'anchor',conversationId:'chat',actorId:'human'},{scope:'group',limit:1});
 await started;await runtime.stop();release();await assert.rejects(pending,{code:'bot-not-connected'});
});

test('checked replies render markdown cards, answer in the topic, and accept a bot-authored source', async () => {
  const runtime = new FeishuRuntime({ lark: fakeLark(), appId: 'app', appSecret: 'secret', ownerOpenIds: ['*'],
    consumerMode: 'external-consumer', harness: { async ensureRunning() {} }, state: {} });
  const starting = runtime.start();
  await waitFor(() => FakeWSClient.instances.length === 1);
  FakeWSClient.instances[0].becomeReady(); await starting;
  const source = { message_id: 'message', chat_id: 'chat', sender: { sender_type: 'app', id_type: 'app_id', id: 'cli_ci' } };
  const client = FakeClient.instances[0];
  client.im.v1.message.get = async () => ({ code: 0, data: { items: [source] } });
  const requests = [];
  client.im.v1.message.reply = async request => {
    requests.push(request);
    return { code: 0, data: { message_id: 'reply' } };
  };
  const route = { messageId: 'message', conversationId: 'chat', actorId: 'cli_ci' };
  await runtime.replyChecked(route, '# done', { format: 'markdown', replyInThread: true, uuid: 'receipt-1' });
  assert.equal(requests[0].data.msg_type, 'interactive');
  assert.equal(requests[0].data.reply_in_thread, true);
  assert.equal(requests[0].data.uuid, 'receipt-1');
  assert.deepEqual(JSON.parse(requests[0].data.content),
    { schema: '2.0', body: { elements: [{ tag: 'markdown', content: '# done' }] } });
  await runtime.replyChecked(route, 'plain');
  assert.equal(requests[1].data.msg_type, 'text');
  assert.deepEqual(JSON.parse(requests[1].data.content), { text: 'plain' });
  assert.equal(requests[1].data.reply_in_thread, false);
  await assert.rejects(runtime.replyChecked(route, 'x', { format: 'html' }), { code: 'bad-request' });
  await assert.rejects(runtime.replyChecked(route, 'x', { replyInThread: 'yes' }), { code: 'bad-request' });
  await assert.rejects(runtime.replyChecked({ ...route, actorId: 'ou_human' }, 'x'), { code: 'stale-route' });
  assert.equal(requests.length, 2);
  await runtime.stop();
});

test('checked resources download another message of the retained chat only', async () => {
  const { Readable } = await import('node:stream');
  const runtime = new FeishuRuntime({ lark: fakeLark(), appId: 'app', appSecret: 'secret', ownerOpenIds: ['*'],
    consumerMode: 'external-consumer', harness: { async ensureRunning() {} }, state: {} });
  const starting = runtime.start();
  await waitFor(() => FakeWSClient.instances.length === 1);
  FakeWSClient.instances[0].becomeReady(); await starting;
  const client = FakeClient.instances[0];
  const anchor = { message_id: 'message', chat_id: 'chat', msg_type: 'text', create_time: '1790830000000',
    body: { content: '{"text":"hi"}' }, sender: { sender_type: 'user', id_type: 'open_id', id: 'human' } };
  const parent = { message_id: 'parent', chat_id: 'chat', msg_type: 'image', create_time: '1790820000000',
    body: { content: '{"image_key":"img_parent"}' }, sender: { sender_type: 'user', id_type: 'open_id', id: 'human' } };
  const elsewhere = { ...parent, message_id: 'elsewhere', chat_id: 'other' };
  client.im.v1.message.get = async ({ path }) => ({ code: 0, data: { items:
    [anchor, parent, elsewhere].filter(item => item.message_id === path.message_id) } });
  const requests = [];
  client.request = async config => {
    requests.push(config.url);
    config.validateStatus(200);
    const body = Buffer.from([0x89, 0x50, 0x4e, 0x47]);
    return { data: Readable.from([body]), headers: new Map([['content-length', String(body.length)]]) };
  };
  const identity = { botId: 'bot', appId: 'app', botOpenId: 'bot', fingerprint: 'a'.repeat(64) };
  const route = { messageId: 'message', conversationId: 'chat', actorId: 'human' };
  const downloaded = await runtime.resourceChecked(identity, route,
    { fileKey: 'img_parent', type: 'image' }, { messageId: 'parent' });
  assert.equal(downloaded.messageId, 'parent');
  assert.equal(requests.length, 1);
  assert.match(requests[0], /messages\/parent\/resources\/img_parent/);
  // A target outside the retained conversation, or a route that no longer names its own message, reaches nothing.
  await assert.rejects(runtime.resourceChecked(identity, route, { fileKey: 'img_parent', type: 'image' }, { messageId: 'elsewhere' }),
    { code: 'stale-route' });
  await assert.rejects(runtime.resourceChecked(identity, { ...route, actorId: 'someone-else' },
    { fileKey: 'img_parent', type: 'image' }, { messageId: 'parent' }), { code: 'stale-route' });
  await assert.rejects(runtime.resourceChecked(identity, route, { fileKey: 'img_parent', type: 'image' }, { messageId: '' }),
    { code: 'bad-request' });
  assert.equal(requests.length, 1);
  await runtime.stop();
});

test('checked chat list reads every joined chat the bot can see', async () => {
  const runtime = new FeishuRuntime({ lark: fakeLark(), appId: 'app', appSecret: 'secret', ownerOpenIds: ['*'],
    consumerMode: 'external-consumer', harness: { async ensureRunning() {} }, state: {} });
  const starting = runtime.start();
  await waitFor(() => FakeWSClient.instances.length === 1);
  FakeWSClient.instances[0].becomeReady(); await starting;
  const client = FakeClient.instances[0];
  const pages = [
    { code: 0, data: { items: [{ chat_id: 'oc_one', name: ' 研发群 ' }, { chat_id: 'oc_noname' }], has_more: true, page_token: 'next' } },
    { code: 0, data: { items: [{ chat_id: 'oc_two', name: '产品群' }], has_more: false } },
  ];
  const calls = [];
  client.im.v1.chat = { list: async request => { calls.push(request); return pages[calls.length - 1]; } };
  const identity = { botId: 'bot', appId: 'app', botOpenId: 'bot', fingerprint: 'a'.repeat(64) };
  assert.deepEqual(await runtime.chatListChecked(identity), { version: 1, chats: [
    { chatId: 'oc_one', name: '研发群' }, { chatId: 'oc_noname' }, { chatId: 'oc_two', name: '产品群' }] });
  assert.deepEqual(calls.map(call => call.params.page_token ?? null), [null, 'next']);
  await runtime.stop();
});
