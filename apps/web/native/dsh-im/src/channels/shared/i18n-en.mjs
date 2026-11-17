import imageInput from './i18n-en/image-input.mjs';
import permission from './i18n-en/permission.mjs';
import diagnostics from './i18n-en/diagnostics.mjs';
// English translations for host-side user-facing text.
// Keys are the exact Chinese literals passed to t() in src/channels/**.
// Chinese output is the identity default and needs no entries here.
// Entries are maintained per area in ./i18n-en/*.mjs and merged here.

import sharedA from './i18n-en/shared-a.mjs';
import sharedB from './i18n-en/shared-b.mjs';
import sharedC from './i18n-en/shared-c.mjs';
import sharedD from './i18n-en/shared-d.mjs';
import feishu from './i18n-en/feishu.mjs';
import dingtalk from './i18n-en/dingtalk.mjs';
import wecom from './i18n-en/wecom.mjs';
import qq from './i18n-en/qq.mjs';
import weixin from './i18n-en/weixin.mjs';
import email from './i18n-en/email.mjs';

export const EN = Object.freeze(Object.assign(
  {},
  diagnostics,
  imageInput,
  permission,
  sharedA,
  sharedB,
  sharedC,
  sharedD,
  feishu,
  dingtalk,
  wecom,
  qq,
  weixin,
  email,
));
