import { createHash } from 'node:crypto';

/** Tenant tokens kept per app identity; the bot identity is still read on every verification. */
const tokenCache = new Map();
const TOKEN_TTL_MS = 10 * 60_000;
const tokenKey = (domain, appId, appSecret) =>
  `${domain}:${appId}:${createHash('sha256').update(appSecret).digest('hex')}`;

function endpointFor(domain, path) {
  const origin = domain === 'lark' ? 'https://open.larksuite.com' : 'https://open.feishu.cn';
  return new URL(path, origin);
}

function jsonResponse(body, operation) {
  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    throw new Error(`${operation} returned a non-JSON response`);
  }
  if (body.code !== 0) {
    throw new Error(`${operation} failed: ${body.msg || `code ${body.code}`}`);
  }
  return body;
}

/** Validate freshly provisioned credentials and read the bot identity. */
export async function verifyFeishuApp({
  appId,
  appSecret,
  domain = 'feishu',
  httpInstance,
  timeoutMs = 15000,
  cache = false,
}) {
  if (!appId || !appSecret) throw new Error('Feishu credentials are incomplete');
  if (!httpInstance || typeof httpInstance.request !== 'function') {
    throw new TypeError('Feishu verification requires an HTTP instance');
  }
  const requestToken = async () => jsonResponse(await httpInstance.request({
    method: 'POST',
    url: endpointFor(domain, '/open-apis/auth/v3/tenant_access_token/internal').href,
    headers: { 'content-type': 'application/json; charset=utf-8' },
    data: { app_id: appId, app_secret: appSecret },
    signal: AbortSignal.timeout(timeoutMs),
    timeout: timeoutMs,
  }), 'Feishu authentication');
  const key = cache ? tokenKey(domain, appId, appSecret) : null;
  const cached = key ? tokenCache.get(key) : undefined;
  let token;
  if (cached && cached.expiresAt > Date.now()) {
    token = cached.token;
  } else {
    const tokenBody = await requestToken();
    if (!tokenBody.tenant_access_token) {
      throw new Error('Feishu authentication returned no tenant access token');
    }
    token = tokenBody.tenant_access_token;
    if (key) {
      const expire = Number(tokenBody.expire);
      const ttl = Number.isFinite(expire) && expire > 120 ? Math.min((expire - 60) * 1000, TOKEN_TTL_MS) : TOKEN_TTL_MS;
      tokenCache.set(key, { token, expiresAt: Date.now() + ttl });
    }
  }

  const readBot = async (value) => jsonResponse(await httpInstance.request({
    method: 'GET',
    url: endpointFor(domain, '/open-apis/bot/v3/info/').href,
    headers: { authorization: `Bearer ${value}` },
    signal: AbortSignal.timeout(timeoutMs),
    timeout: timeoutMs,
  }), 'Feishu bot verification');
  let botBody;
  try {
    botBody = await readBot(token);
  } catch (error) {
    // A cached token can be revoked early; drop it and retry once with a fresh one.
    if (!key || !/invalid|expired|99991663|99991661|99991668/i.test(String(error?.message))) throw error;
    tokenCache.delete(key);
    const tokenBody = await requestToken();
    if (!tokenBody.tenant_access_token) throw new Error('Feishu authentication returned no tenant access token');
    token = tokenBody.tenant_access_token;
    const expire = Number(tokenBody.expire);
    const ttl = Number.isFinite(expire) && expire > 120 ? Math.min((expire - 60) * 1000, TOKEN_TTL_MS) : TOKEN_TTL_MS;
    tokenCache.set(key, { token, expiresAt: Date.now() + ttl });
    botBody = await readBot(token);
  }
  const bot = botBody.bot ?? {};
  return Object.freeze({
    appId,
    name: bot.app_name ?? bot.bot_name ?? null,
    openId: bot.open_id ?? null,
    activated: bot.activate_status ?? null,
  });
}
