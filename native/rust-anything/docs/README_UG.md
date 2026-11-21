<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="rust-anything قىسقۇچپاقىسى">
</p>

<h1 align="center">rust-anything</h1>

<div dir="rtl">

<p align="center"><b>ئەپىڭىزنى Rust دا قايتا يېزىڭ، ھەمدە ئۇنىڭ يەنىلا ئوخشاش ئىشلەيدىغانلىقىنى ئىسپاتلاڭ.</b></p>

<p align="center">2.0 دىكى يېڭىلىق: <a href="real-world.md#prettier">Prettier، Rust قا كۆچۈرۈلدى</a>. 19x تېز · خاتىرىلەنگەن 17,224 ئىجرانىڭ 15,837 تامامەن Rust دا · ھەممىسى بايتمۇ-بايت ئوخشاش</p>

<p align="center">
  <a href="https://crates.io/crates/rust-anything"><img src="https://img.shields.io/crates/v/rust-anything?color=E2862A&label=crates.io" alt="crates.io نەشرى"></a>
  <a href="https://www.npmjs.com/package/rust-anything"><img src="https://img.shields.io/npm/v/rust-anything?color=E2862A&label=npm" alt="npm نەشرى"></a>
  <a href="https://github.com/erkamyaman/rust-anything/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/erkamyaman/rust-anything/ci.yml?branch=main&label=tests" alt="سىناقلار"></a>
  <a href="https://github.com/erkamyaman/rust-anything"><img src="https://img.shields.io/github/stars/erkamyaman/rust-anything?style=social" alt="GitHub يۇلتۇزلىرى"></a>
  <a href="https://github.com/AIshihzong/awesome-reverse-engineer-anything"><img src="https://img.shields.io/badge/Featured%20in-Awesome%20REA-f0b429" alt="Featured in Awesome REA"></a>
</p>

<p align="center">
  <a href="#تېز-باشلاش">تېز باشلاش</a> ·
  <a href="#تىللار">تىللار</a> ·
  <a href="real-world.md">ئەمەلىي كۆچۈرمىلەر</a> ·
  <a href="how-it-works.md#how-it-works">قانداق ئىشلەيدۇ</a> ·
  <a href="whole-apps.md#port-a-whole-app">پۈتۈن ئەپلەر</a> ·
  <a href="faq.md#faq">دائىم سورىلىدىغان سوئاللار</a>
</p>

<p align="center">تۈركىيەدە 🇹🇷 <a href="https://github.com/erkamyaman">erKam Yaman</a> ياسىغان · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ ماڭا ھامىي بولۇڭ</a></p>

<p align="center">ھامىي: <a href="https://coderabbit.ai"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

</div>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · ئۇيغۇرچە · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a></sub></p>
<p align="center"><sub>بۇ تەرجىمىنى جامائەت ئاسرايدۇ، شۇڭا ئۇ <a href="../README.md">ئىنگلىزچە README</a> دىن ئارقىدا قېلىشى مۇمكىن.</sub></p>

<div dir="rtl">

<table>
  <tr>
    <td width="25%" valign="top">
      <b>خاتىرىلەش</b><br>
      <code>record</code> ئەسلى نۇسخىنى ھاسىل قىلىنغان چېگرا ئەھۋاللار، مەنە تۇزاقلىرى ۋە ئۆزىڭىزنىڭ كىرگۈزۈشلىرىدە ئىجرا قىلىپ، ئۇنىڭ قايتۇرغان ياكى تاشلىغان نەرسىسىنى ساقلايدۇ.
    </td>
    <td width="25%" valign="top">
      <b>كۆچۈرۈش</b><br>
      <code>bridge</code> Rust crate ۋە ئوراپ تۇرغۇچنى ھاسىل قىلىدۇ، شۇڭا چاقىرغۇچىلار ئەسلى API نى ساقلاپ قالىدۇ. Rust كودىنى سىز ياكى ئاگېنتىڭىز يازىدۇ.
    </td>
    <td width="25%" valign="top">
      <b>تەكشۈرۈش</b><br>
      <code>diff</code> خاتىرىلەنگەن ھەر بىر ئەھۋالنى Rust تا قايتا قويىدۇ، ھەممىسى ماس كېلىپ <code>gate</code> ئۆتمىگۈچە ھېچنېمە ئېلان قىلىنمايدۇ.
    </td>
    <td width="25%" valign="top">
      <b>Lift</b><br>
      قايتا قويۇش زاپاس يولسىز ئۆتسە ۋە ھەر بىر بىرلىك gate دىن ئۆتسە، <code>app lift</code> ئۇنى ساپ Rust ئەپىگە ئايلاندۇرىدۇ. ئەسلى نۇسخىغا ھەرگىز تېگىلمەيدۇ.
    </td>
  </tr>
</table>

**پۈتۈن ئەپىڭىزنى Rust دا پارچىمۇپارچە قايتا يېزىڭ.** rust-anything سىزنىڭ Node.js (TypeScript/JavaScript)،
Python، C، Go، Ruby ياكى Java ئەپىڭىزنىڭ بۈگۈن نېمە قىلىدىغانلىقىنى خاتىرىلەيدۇ، ئاندىن ئالدىغا ئىلگىرىلەشتىن بۇرۇن
Rust دا قايتا يېزىلغان ھەر بىر پارچىنى شۇ خاتىرە بىلەن سېلىشتۇرۇپ تەكشۈرىدۇ، ئەپ ساپ Rust بولغۇچە. CLI، HTTP
مۇلازىمېتىرلىرى ۋە كۇتۇپخانىلار؛ Rust كودىنى سىزنىڭ پروگرامما ئاگېنتىڭىز (ياكى سىز ئۆزىڭىز) يازىدۇ. ئۇ پەقەت
خاتىرە قاپلىغان نەرسىلەرنىلا تۇتالايدۇ: [ئۇ نېمىلەرنى تۇتالمايدۇ](how-it-works.md#what-it-cant-catch) غا قاراڭ. پۈتۈن قوراللار ۋە مۇلازىمېتىرلار، ياكى پەقەت ئاستا قىسمى،
ھەر قېتىمدا بىر ئىسسىق يول (hot path):

| ھەقىقىي تۈر، چىقىرىلمىسى ئەسلى نۇسخا بىلەن ئوخشاش | ئەسلى نۇسخا | rust-anything بىلەن |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2، بۇيرۇق قۇرىدىن 2,360 JS/TS ھۆججىتى | 128.6 s | **6.9 s: 19x تېز** |
| Prettier، بۇيرۇق قۇرىدىن 6,305 Markdown ھۆججىتى | 352 s | **22.5 s: 15.6x تېز** |
| [json-server](real-world.md#json-server)، پۈتۈن بىر Node مۇلازىمېتىرى: بىكار تۇرغاندىكى ئىچكى ساقلىغۇچ | 74 MB | **3.5 MB**، بىرلا 5.3 MB ئىككىلىك ھۆججەت |
| json-server، قوزغىتىشتىن تۇنجى جاۋابقىچە | 79 دىن 85 ms غىچە | **8 دىن 10 ms غىچە** |
| [Fuse.js](real-world.md#fusejs) دا 50,000 قۇر ئىچىدە ئىزدەش، ھەر سوئالغا | 91.2 ms | **2.8 ms: 32x تېز** |

Prettier: خاتىرىلەنگەن 17,224 ئىجرانىڭ 15,837 سى 10 تىلدا (JavaScript، TypeScript، JSX، JSON، GraphQL،
YAML، Markdown، CSS، SCSS، Less) تولۇق Rust دا فورماتلىنىدۇ، 17,224 نىڭ ھەممىسى Prettier بىلەن بايتمۇبايت
ماس كېلىدۇ، چۈنكى تېخى كۆچۈرۈلمىگەن ھەر قانداق نەرسە Prettier غا قايتىدۇ (fallback). json-server: خاتىرىلەنگەن
514/514 HTTP ئەھۋالى ساپ Rust ئىككىلىك ھۆججىتىدە ماس كېلىدۇ. Apple M4 دا ئۆلچەندى، ھەر بىرى ئىككى قېتىم ئىجرا
قىلىندى، ھەر بىر چىقىرىلما ۋاقىت ئۆلچەشتىن بۇرۇن سېلىشتۇرۇلدى ([تەپسىلاتى](real-world.md)). ھەر بىر كىچىك
فۇنكسىيە ئۈچۈن Rust نى ئايرىم چاقىرىش ئادەتتە JIT تىن *ئاستا*؛ rust-anything كۆچۈرۈشكە ئەرزىيدىغان ئايلانمىنى
تاپىدۇ ۋە ئۇنى كۆچۈرىدۇ.

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier ۋە ئۇنىڭ Rust كۆچۈرمىسى ئوخشاش 59 ھەقىقىي ھۆججەتنى فورماتلايدۇ، ھەر بىر ھۆججەتكە بىر جەريان: Prettier غا 3.63 s، Rust كۆچۈرمىسىگە 0.20 s كېتىدۇ، چىقىرىلمىسى بايتمۇبايت ئوخشاش." width="820">
  <br>
  <sub>ms، Fuse.js ۋە json-server دىن ئېلىنغان 59 ھەقىقىي ھۆججەتتە Prettier بىلەن ئۇنىڭ Rust كۆچۈرمىسى (Apple M4). چىقىرىلمىسى بايتمۇبايت ئوخشاش. <a href="real-world.md#prettier">قانداق ئۆلچەنگەن</a>.</sub>
</p>

<p align="center">
  <img src="assets/json-server.gif" alt="json-server بىر Node مۇلازىمېتىرى ۋە بىرلا Rust ئىككىلىك ھۆججىتى سۈپىتىدە: بىكار تۇرغاندىكى ئىچكى ساقلىغۇچ 74 MB ۋە 3.5 MB، قوزغىتىشتىن تۇنجى جاۋابقىچە 79 دىن 85 ms غىچە ۋە 8 دىن 10 ms غىچە، خاتىرىلەنگەن 514 HTTP ئەھۋالىنىڭ 514 ى ماس كېلىدۇ." width="820">
  <br>
  <sub>پۈتۈن بىر Node مۇلازىمېتىرى بولغان json-server ساپ Rust ئىككىلىك ھۆججىتىگە ئايلاندۇرۇلدى (Apple M4). <a href="real-world.md#json-server">قانداق ئۆلچەنگەن</a>.</sub>
</p>

> rust-anything مۇستەقىل تۈر، ئۇ Rust Project ياكى Rust Foundation بىلەن مۇناسىۋەتلىك ئەمەس ۋە ئۇلار تەرىپىدىن تەستىقلانمىغان.
> "Rust" بولسا Rust Foundation نىڭ سودا ماركىسى.

> [!WARNING]
> `record` ۋە `app record` ئەسلى كودىڭىزنى ئىجرا قىلىدۇ، ھەر بىر فۇنكسىيەنى (ياكى پۈتۈن ئەپنى) نۇرغۇن ھاسىل
> قىلىنغان كىرگۈزۈشلەردە چاقىرىدۇ، `app record` بولسا ھەر بىر ئەھۋالنى ئىككى قېتىم ئىجرا قىلىدۇ. `diff`، `gate` ۋە `app replay`
> ئوخشاش ئەھۋاللارنى كۆچۈرمە ياكى ئەسلى نۇسخىدا قايتا ئىجرا قىلىدۇ. ھۆججەت يازىدىغان، ساندان بىلەن ئالاقىلىشىدىغان
> ياكى تورنى چاقىرىدىغان كود بۇنى ھەر بىر ئەھۋال ئۈچۈن بىر قېتىم قىلىدۇ. ئۇلارنى تۈرنىڭ نۇسخىسىدا، سىناق ساندانى
> بىلەن، مۇھىتتا ئىشلەپچىقىرىش كىنىشكىلىرى بولمىغان ھالدا ئىجرا قىلىڭ.

## تېز باشلاش

بۇنى Claude Code، Codex، Cursor، Gemini CLI ياكى OpenCode غا چاپلاڭ، ئاگېنتىڭىز ئۇنى ئورناتسۇن:

</div>

```text
rust-anything نى ئورنات: `cargo install rust-anything` نى ئىجرا قىل، ئاندىن `rust-anything setup` نى.
سېنى قاچان قايتا قوزغىتىشىم كېرەكلىكىنى ماڭا ئېيت.
```

<div dir="rtl">

ياكى ئۆزىڭىز:

</div>

```sh
cargo install rust-anything   # crates.io دىن
rust-anything setup           # ئۇنى Claude Code، Codex، Cursor، Gemini CLI ۋە OpenCode دا تىزىملايدۇ
```

<div dir="rtl">

تېخى Rust toolchain يوقمۇ؟ macOS، Linux ۋە Windows ئۈچۈن تەييار ئىككىلىك ھۆججەتلەر ھەر بىر
[GitHub release](https://github.com/erkamyaman/rust-anything/releases) غا قوشۇپ قويۇلغان، JavaScript
تۈرلىرى ئۈچۈن `npm install -g rust-anything` ئوخشاش ئىككىلىك ھۆججەتنى ئورنىتىدۇ.

Claude Code دا ئۇنى قىستۇرما (plugin) سۈپىتىدىمۇ ئورناتقىلى بولىدۇ (MCP مۇلازىمېتىرى ۋە skill؛ يۇقىرىدىكى CLI
`PATH` ىڭىزدا بولۇشى كېرەك):

</div>

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

<div dir="rtl">

`setup` تاپقان ھەر بىر ئاگېنتقا MCP مۇلازىمېتىرىنى قوشىدۇ ۋە workflow skill نى ئاگېنت skill لارنى ئوقۇيدىغان
ئورۇنغا ئورنىتىدۇ. ئۇ باشقا تەڭشەكلىرىڭىزنى ساقلايدۇ، قايتا ئىجرا قىلىش بىخەتەر، `--remove` بولسا ئۇنى ئەمەلدىن قالدۇرىدۇ.
ئاندىن ئاگېنتىڭىزدىن سوراڭ، مەسىلەن:

- *"بۇ CLI نى rust-anything ئارقىلىق باشتىن ئاخىرىغىچە Rust دا قايتا يېزىپ، ئۇنىڭ ئوخشاش ئىشلەيدىغانلىقىنى ئىسپاتلا."*
- *"بۇ ئىزدەش ئاستىدەك تۇرىدۇ. ئۇنى rust-anything بىلەن profile قىلىپ، ئىسسىق ئايلانمىنى Rust قا يۆتكە."*
- *"rust-anything بىلەن `src/parser/` نى Rust قا كۆچۈرۈشنى پىلانلا؛ بىرىنچى بىرلىكنى كۆچۈرۈپ ئىسپاتلا."*
- *"`rust/text-rs` دىكى Rust كۆچۈرمىسىنى ئەسلى نۇسخا بىلەن benchmark دا سېلىشتۇرۇپ، ئېلان قىلىشقا بولامدۇ ئېيت."*

### ماس كېلىدىغان ئاگېنتلار

| ئاگېنت | `rust-anything setup` نېمە قوشىدۇ |
| --- | --- |
| Claude Code | `~/.claude.json` دا MCP مۇلازىمېتىرى، `~/.claude/skills/` دا skill |
| Codex | `~/.codex/config.toml` دا MCP مۇلازىمېتىرى، `~/.codex/skills/` دا skill |
| Cursor | `~/.cursor/mcp.json` دا MCP مۇلازىمېتىرى |
| Gemini CLI | `~/.gemini/settings.json` دا MCP مۇلازىمېتىرى |
| OpenCode | `~/.config/opencode/opencode.json` دا MCP مۇلازىمېتىرى (`mcp.servers`)، `~/.config/opencode/skills/` دا skill |
| باشقا ھەر قانداق MCP خېرىدارى | `rust-anything mcp` نى stdio مۇلازىمېتىرى سۈپىتىدە قولدا قوشۇڭ ([ئورنىتىش](install.md#install) قا قاراڭ) |

مەۋجۇت ھۆججەتنى ئۆزگەرتىشتىن بۇرۇن، `setup` ئۇنىڭ يېنىغا بىر نۇسخىسىنى ساقلايدۇ (`<file>.rust-anything.bak`).

OpenCode نى قولدا ئۇلاش ئۈچۈن مۇلازىمېتىرنى `~/.config/opencode/opencode.json` غا قويۇڭ (`command` پۈتۈن argv)
ۋە [`skills/rust-anything/`](../skills/rust-anything) نى `~/.config/opencode/skills/rust-anything/` غا كۆچۈرۈڭ:

</div>

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

<div dir="rtl">

ياكى قەدەملەرنى ئۆزىڭىز ئىجرا قىلىڭ:

</div>

```sh
rust-anything profile . -- node bench.mjs              # نېمە ئىسسىق، ئۇنى كىم چاقىرىدۇ
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# src/port.rs نى يېزىڭ (ياكى ئاگېنتىڭىزغا يازدۇرۇڭ)، ئاندىن:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # خاتىرىلەنگەن ھەر بىر ئەھۋال ماس كېلىشى كېرەك
rust-anything bench rust/fuzzy-rs                      # تېزمۇ؟ ھۆكۈم ئېيتىدۇ
rust-anything gate rust/fuzzy-rs                       # بۇ ئۆتسە، ئىش تامام
```

<div dir="rtl">

rust-anything كودنى تەرجىمە قىلمايدۇ: Rust كودىنى سىزنىڭ پروگرامما ئاگېنتىڭىز (Claude Code، Codex، Cursor، OpenCode، ...) ياكى
سىز ئۆزىڭىز يازىسىز. rust-anything ئاگېنت ئىشەنچلىك پەرەز قىلالمايدىغان نەرسىلەرنى تەمىنلەيدۇ: ۋاقىت قەيەرگە كېتىدۇ، نېمىنى
ئالدى بىلەن ۋە قايسى تەرتىپتە كۆچۈرۈش، قالغان كود Rust نى API ئۆزگەرمىگەن ھالدا چاقىرالىشى ئۈچۈن bridge،
كۆچۈرمە ماس كېلىشى كېرەك بولغان ئېنىق ھەرىكەت، ئۇنى ئېلان قىلىشنىڭ بىخەتەر ئۇسۇلى ۋە كۆچۈرمىنىڭ قاچان
تامام بولغانلىقىنى ئېيتىدىغان gate.

ئوخشاش چىقىرىلما يېتەرلىك ئەمەس. ئەپنىڭ سىرتقا نېمە ئەۋەتىدىغانلىقىنىمۇ خاتىرىلەڭ، شۇندا ئېلخەتنى ئىككى قېتىم
ئەۋەتىدىغان قايتا يېزىلغان نۇسخا مەغلۇپ بولىدۇ:

<p align="center">
  <img src="assets/effects.gif" alt="--effects smtp بىلەن app record ھەر بىر تىزىملىتىش ئۈچۈن بىر قارشى ئېلىش ئېلخېتىنى خاتىرىلەيدۇ؛ ئوخشاش چىقىرىلما بېرىدىغان، ئەمما ئېلخەتنى ئىككى قېتىم ئەۋەتىدىغان قايتا يېزىلغان نۇسخىدا app replay 0/2 ئەھۋال ئۆتكەنلىكىنى ۋە effects تا مەغلۇپ بولغانلىقىنى بىلدۈرىدۇ، دوكلاتتا ئارتۇق ئېلخەت كۆرۈنىدۇ." width="820">
  <br>
  <sub><code>app record --effects</code> ۋە <code>app replay</code> نىڭ ھەقىقىي ئىجراسى. <a href="whole-apps.md#side-effects-email-webhooks-and-databases">يانداش تەسىرلەر</a> گە قاراڭ.</sub>
</p>

## تىللار

| تىل | بىرلىك | Bridge | خاتىرىلىگۈچى ۋە قايتا قويغۇچى |
| --- | --- | --- | --- |
| TypeScript/JavaScript | قىسقۇچ ياكى ھۆججەت | ھاسىل قىلىنغان ئىككىلىك wire فورماتلىق napi-rs ياكى wasm-bindgen؛ كلاس ۋە async مۇ بار | Node، `.ts` مەنبە ھۆججەتلىرىنى ئىجرا قىلىپ |
| Python | بوغچا ياكى مودۇل | PyO3 (abi3)، بىۋاسىتە ئايلاندۇرۇش، تىپلىق مۇستەسنالار | Python، ئەسلى نۇسخىنى import قىلىپ |
| C | `.c` ھۆججىتى | ئەسلى ناملىق `extern "C"` فۇنكسىيەلىرى ۋە header | ئورتاق كۇتۇپخانىغا تەرجىمە قىلىنغان ئەسلى نۇسخا، ctypes ئارقىلىق چاقىرىلىدۇ، ھەر ئەھۋالغا بىر fork |
| Go | بوغچا | تىپلىق cgo چاقىرىقلىرى ۋە ھاسىل قىلىنغان ئىككىلىك wire فورماتى، ئەسلى API لىق بوغچىدا | ھاسىل قىلىنغان Go قوزغاتقۇچىسى، ئايرىم مودۇل سۈپىتىدە قۇرۇلىدۇ |
| Ruby | مودۇل فۇنكسىيەلىرى بار ھۆججەت ياكى قىسقۇچ | ئەسلى مودۇللار ئاستىدىكى magnus كېڭەيتمىسى | ھاسىل قىلىنغان Ruby قوزغاتقۇچىسى |
| Java | `public static` مېتودلىرى بوغچىسى | تىپلىق JNI ۋە ھاسىل قىلىنغان ئىككىلىك wire فورماتى، ئەسلى مېتودلىق wrapper كلاسلىرىدا | ھاسىل قىلىنغان reflection قوزغاتقۇچىسى |
| PHP | فۇنكسىيەلەر ۋە `public static` مېتودلىرى قىسقۇچى | كېيىنكى قەدەم: C ABI ئۈستىدە PHP FFI | بۈگۈندىلا پۈتۈن ئەپلەر: `app record`، `app replay` ۋە front door، CLI ۋە مۇلازىمېتىرلار ئۈچۈن (`php -S`) |
| C# | `public static` مېتودلىرى قىسقۇچى | كېيىنكى قەدەم: C ABI ئۈستىدە P/Invoke | بۈگۈندىلا پۈتۈن ئەپلەر: `app record`، `app replay` ۋە front door، CLI ۋە ASP.NET مۇلازىمېتىرلىرى ئۈچۈن (`dotnet <app>.dll`) |

C دىن باشقا ھەر بىر تىل batch كلاسلىرىغىمۇ (`--batch`) ئېرىشىدۇ: ئېلېمېنتلار بىر قېتىم ئۆتۈپ
Rust تا قالىدۇ، ھەر بىر چاقىرىق ئۇلارنىڭ ھەممىسىنى بارلىق يادرولاردا بىر تەرەپ قىلىدۇ.

تىل ئامباردىن (repository) بايقىلىدۇ (`go.mod`، `pom.xml`، `.csproj` ياكى `.sln`،
`pyproject.toml`، `Gemfile`، `composer.json`، `package.json`، بولمىسا ئەڭ كۆپ ئۇچرايدىغان مەنبە ھۆججەتلەر)؛
`--lang` ئۇنى قاپلايدۇ. تىپلار مەنبە كودتىن ئېلىنىدۇ: TS تىپلىرى، Python type hints، C، Go، Java ۋە C#
ئىمزالىرى، Ruby YARD بەلگىلىرى، PHP تىپلىرى ۋە phpdoc؛ سان سۈپىتىدە ئىشلىتىلگەن تىپسىز پارامېتىرلار
سان دەپ پەرەز قىلىنىدۇ.

PHP ئۈچۈن، `analyze` PHP ئەپىنى پىلانلايدۇ (Composer بوغچىلىرى ۋە PSR-4 autoloading، بوش
سېلىشتۇرۇش ۋە پۈتۈن سانلارنىڭ float قا تېشىپ كېتىشىگە ئوخشاش خەتەرلەر، Composer بوغچىلىرىغا ماس crate لار)،
PHP CLI ياكى مۇلازىمېتىرىنى خاتىرىلەپ، قايتا قويۇپ، Rust front door نىڭ ئارقىسىغا قويغىلى بولىدۇ. ئۇنىڭ
بىرلىكلىرىنى PHP FFI ئارقىلىق Rust قا كۆچۈرۈش كېيىنكى قەدەم ([issue #2](https://github.com/erkamyaman/rust-anything/issues/2)).

C# مۇ ئوخشاشلا ئىشلەيدۇ: `analyze` `.csproj` تۈرلىرى ۋە ئۇلارنىڭ NuGet بوغچىلىرىنى ئوقۇيدۇ
(crate لارغا ماسلاشتۇرۇپ)، `using` كۆرسەتمىلىرىنى قىسقۇچلارغا باغلايدۇ ۋە C# نىڭ ئوخشىمايدىغان جايلىرىنى بەلگىلەيدۇ
(`decimal`، `dynamic`، LINQ نىڭ كېچىكتۈرۈلگەن سوئاللىرى، `Task`، `lock`، reflection). C# CLI ياكى ASP.NET
مۇلازىمېتىرىنى خاتىرىلەپ، قايتا قويۇپ، Rust front door نىڭ ئارقىسىغا قويغىلى بولىدۇ؛ بىرلىكلەرنى P/Invoke ئارقىلىق كۆچۈرۈش
كېيىن كېلىدۇ.

## تېخىمۇ كۆپ ئۆگىنىش

- [قانداق ئىشلەيدۇ](how-it-works.md): ئايلانما، `bridge` ۋە `analyze` نېمىلەرنى بەلگىلەيدۇ، ئىسسىق يوللار، ئۇ نېمىلەرنى تۇتالمايدۇ، ھەقىقىي كۆچۈرمە نېمىلەرنى تەلەپ قىلىدۇ، ۋە ئەسلى نۇسخىغا ئەگىشىپ مېڭىش (`map`)
- [پۈتۈن ئەپنى كۆچۈرۈش](whole-apps.md): مۇلازىمېتىرلار، CLI ۋە كۇتۇپخانىلار، front door دىن `app lift` قىچە
- [مىساللار ۋە ئەمەلىي تەتقىقاتلار](examples.md): x18n، ۋە ئالتە تىلدىكى ھەقىقىي كود
- [ئەمەلىي كۆچۈرمىلەر](real-world.md): json-server، Fuse.js، vercel/ms، benchmark لىرى بىلەن
- [ئورنىتىش، يېڭىلاش ۋە تەرەققىي قىلدۇرۇش](install.md): بارلىق ئورنىتىش ئۇسۇللىرى، Windows، ۋە مەنبە كودىدىن قۇرۇش
- [دائىم سورىلىدىغان سوئاللار](faq.md) · [ھالەت](STATUS.md) · [MCP قوراللىرى](tools.md)

## ھامىيلار

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> ھەقسىز ۋە ئوچۇق كودلۇق. ئەگەر ئۇ خىزمىتىڭىزگە ياردەم قىلسا، ئۇنى قوللاشنى ئويلىشىپ بېقىڭ.<br>
  سىزنىڭ قوللىشىڭىز تەرەققىياتنى داۋاملاشتۇرىدۇ.
</p>

<p align="center">
  <a href="https://coderabbit.ai">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="شىركەت ھامىيسى: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="ھامىيلار: Santosh Yadav، Hakan Soytürk، Ali Yilmaz، Ashley Hunter، Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/%DA%BE%D8%A7%D9%85%D9%89%D9%8A_%D8%A8%D9%88%D9%84%DB%87%D8%B4-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="ھامىي بولۇش"></a>
</p>

## تۆھپە قوشۇش

Issue ۋە pull request لار قارشى ئېلىنىدۇ؛ [CONTRIBUTING.md](../CONTRIBUTING.md) سەپلەش، سىناقلارنى قەيەرگە
قويۇش ۋە يېڭى تىلنى قانداق قوشۇشنى چۈشەندۈرىدۇ. لايىھە قارارلىرى ۋە ئۇلارنىڭ سەۋەبلىرى
[DECISIONS.md](../DECISIONS.md) دا. ھەقىقىي كۆچۈرمىلەرنىڭ ئەمەلىي مىساللىرى ئالاھىدە قارشى ئېلىنىدۇ. بىخەتەرلىك
مەسىلىلىرىنى مەخپىي دوكلات قىلىڭ ([SECURITY.md](../SECURITY.md))، ۋە
[ھەرىكەت قائىدىسى](../CODE_OF_CONDUCT.md) گە قاراڭ.

## ئىجازەتنامە

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). ئامبىرىڭىزدىكى كۆچۈرمىلەر ۋە ھاسىل قىلىنغان كود سىزنىڭ.

</div>
