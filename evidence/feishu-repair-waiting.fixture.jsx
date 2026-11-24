// Browser fixture for the Feishu permission-completion waiting view.
// Renders the real production FeishuSettingsTab with an in-memory RPC that
// restores a Host-submitted callback repair, putting the page in the same
// `connecting` state a user sees after confirming Feishu permissions.
import * as React from 'react';
import { createRoot } from 'react-dom/client';
import { FeishuSettingsTab } from '../apps/web/native/dsh-im/plugin-src/client/channels/feishu/index.js';
import { FEISHU_ENDPOINTS } from '../apps/web/native/dsh-im/plugin-src/client/channels/feishu/api.js';
import { installImStyles } from '../apps/web/native/dsh-im/plugin-src/client/styles.js';

installImStyles();

const now = Date.now();
const snapshot = {
  schemaVersion: 2,
  revision: 1,
  state: 'connected',
  bots: [{
    botId: 'bot_target',
    state: 'connected',
    connected: true,
    configured: true,
    workspace: '/workspace/current',
    bot: { name: '目标机器人', appIdMasked: 'cli_tar••••rget' },
    health: { status: 'healthy', summary: '长连接运行正常' },
  }],
  provisioning: {
    attemptId: 'reg_committed',
    operation: 'callback_repair',
    botId: 'bot_target',
    submitted: true,
    expiresAt: now + 60_000,
    pollIntervalMs: 3600_000,
  },
};

const rpcCall = async (endpoint) => {
  if (endpoint === FEISHU_ENDPOINTS.status) return { ok: true, value: snapshot };
  return { ok: true, value: {} };
};

const root = createRoot(document.getElementById('app'));
root.render(React.createElement('div', {
  className: 'dim-page dim-panel',
  style: { width: '820px', padding: '16px' },
}, React.createElement(FeishuSettingsTab, { rpcCall })));
