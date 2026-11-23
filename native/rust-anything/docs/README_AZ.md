<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="rust-anything xərçəngi">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · Azərbaycanca · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a></sub></p>
<p align="center"><sub>Bu tərcümə icma tərəfindən dəstəklənir və <a href="../README.md">ingiliscə README</a>-dən geri qala bilər.</sub></p>

<p align="center"><b>Tətbiqinizi Rust-da yenidən yazın və onun hələ də eyni davrandığını sübut edin.</b><br>
Kodu özü tərcümə etmir: Rust-u kodlaşdırma agentiniz (və ya siz) yazır, rust-anything isə onu orijinalla yoxlayır.</p>

<p align="center">
  <img src="https://img.shields.io/github/languages/top/erkamyaman/rust-anything?logo=rust&label=Rust&color=CE422B" alt="Rust share of the code">
  <img src="https://img.shields.io/badge/macOS%20·%20Linux%20·%20Windows-native-2563EB" alt="Native binaries for macOS, Linux and Windows">
  <a href="../LICENSE"><img src="https://img.shields.io/badge/license-MIT-555" alt="MIT license"></a>
  <a href="https://crates.io/crates/rust-anything"><img src="https://img.shields.io/crates/v/rust-anything?color=E2862A&label=crates.io" alt="crates.io version"></a>
  <a href="https://www.npmjs.com/package/rust-anything"><img src="https://img.shields.io/npm/v/rust-anything?color=E2862A&label=npm" alt="npm version"></a>
  <br>
  <a href="https://coderabbit.link/erkamyaman"><img src="https://img.shields.io/coderabbit/prs/github/erkamyaman/rust-anything?labelColor=171717&color=FF570A&label=Reviewed+by+CodeRabbit" alt="Pull requests reviewed by CodeRabbit"></a>
  <a href="https://github.com/rust-unofficial/awesome-rust#development-tools"><img src="https://awesome.re/mentioned-badge.svg" alt="Mentioned in Awesome Rust"></a>
  <a href="https://github.com/AIshihzong/awesome-reverse-engineer-anything"><img src="https://img.shields.io/badge/Featured%20in-Awesome%20REA-f0b429" alt="Featured in Awesome REA"></a>
  <a href="https://github.com/erkamyaman/rust-anything"><img src="https://img.shields.io/github/stars/erkamyaman/rust-anything?style=social" alt="GitHub stars"></a>
</p>

<p align="center"><a href="https://github.com/erkamyaman/rust-anything/discussions/127"><img src="https://img.shields.io/badge/Join%20the%20discussion-6E40C9?style=for-the-badge&logo=github&logoColor=white" alt="Join the discussion"></a></p>

<p align="center"><a href="https://trendshift.io/repositories/363468" target="_blank" rel="noopener noreferrer"><img src="https://trendshift.io/api/badge/trendshift/repositories/363468/daily?language=Rust" alt="erkamyaman/rust-anything: #1 Rust repository of the day on Trendshift" width="250" height="55"/></a></p>

<p align="center"><b><a href="#sürətli-başlanğıc">Sürətli başlanğıc</a></b> · <a href="https://github.com/erkamyaman/rust-anything/releases">Releases</a> · <a href="#sürətli-başlanğıc">Claude Code plugin</a> · <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=rust-anything">MCP Registry</a> · <a href="real-world.md">Real portlar</a> · <a href="faq.md#faq">Tez-tez verilən suallar</a><br>
<sub><a href="#dillər">Dillər</a> · <a href="how-it-works.md#how-it-works">Necə işləyir</a> · <a href="whole-apps.md#port-a-whole-app">Bütöv tətbiqlər</a> · <a href="../LICENSE">MIT</a></sub><br>
Türkiyədə 🇹🇷 <a href="https://github.com/erkamyaman">erKam Yaman</a> tərəfindən hazırlanıb · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ Sponsor olun</a> · Sponsor: <a href="https://coderabbit.link/erkamyaman"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center">2.0-da yeni: <a href="real-world.md#prettier">Prettier, Rust-a port edildi</a>. 19x daha sürətli · 17,224 qeydə alınmış icranın 15,861-si tamamilə Rust-da · hamısı bayt-bayt eyni</p>

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier və onun Rust portu eyni 59 real faylı, hər fayl üçün bir proses olmaqla formatlayır: Prettier 3.63 s, Rust portu 0.20 s çəkir və çıxış bayt-bayt eynidir." width="820">
  <br>
  <sub>ms, Fuse.js və json-server-dən götürülmüş 59 real faylda Prettier onun Rust portuna qarşı (Apple M4). Bayt-bayt eyni çıxış. <a href="real-world.md#prettier">Necə ölçülüb</a>.</sub>
</p>

<p align="center"><sub>PHP və C#: bu gün qeyd, təkrar və front door; vahid portları növbədədir.</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>Qeyd</b><br>
      <code>record</code> orijinalı yaradılmış kənar hallar, semantik tələlər və sizin öz girişlərinizlə işə salır və nə qaytardığını və ya hansı xətanı atdığını saxlayır.
    </td>
    <td width="25%" valign="top">
      <b>Port</b><br>
      <code>bridge</code> Rust crate-ini və wrapper yaradır ki, çağıranlar orijinal API-ni saxlasın. Rust-u siz və ya agentiniz yazır.
    </td>
    <td width="25%" valign="top">
      <b>Yoxlama</b><br>
      <code>diff</code> hər qeydə alınmış halı Rust-a qarşı təkrar oynadır və hamısı uyğun gəlib <code>gate</code> keçənə qədər heç nə buraxılmır.
    </td>
    <td width="25%" valign="top">
      <b>Lift</b><br>
      Təkrar oynatma fallback olmadan keçəndə və hər vahid gate-dən keçəndə, <code>app lift</code> onu təmiz Rust tətbiqinə çevirir. Orijinala heç vaxt toxunulmur.
    </td>
  </tr>
</table>

**Bütün tətbiqinizi Rust-da, hissə-hissə yenidən yazın.** rust-anything Node.js
(TypeScript/JavaScript), Python, C, Go, Ruby və ya Java tətbiqinizin bu gün nə etdiyini qeydə alır,
sonra Rust-da yenidən yazılmış hər hissəni növbətiyə keçməzdən əvvəl bu qeydlə yoxlayır, tətbiq
tamamilə Rust olana qədər. CLI-lar, HTTP serverləri və kitabxanalar; Rust kodunu kodlaşdırma
agentiniz (və ya siz) yazır. O, yalnız qeydin əhatə etdiyini tuta bilər: bax
[Nəyi tuta bilmir](how-it-works.md#what-it-cant-catch). Bütöv alətlər və serverlər, yaxud yalnız
yavaş hissə, hər dəfə bir hot path:

| Real layihə, orijinalla eyni çıxış | Orijinal | rust-anything ilə |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2, komanda sətrindən 2,360 JS/TS faylı | 128.6 s | **6.9 s: 19x daha sürətli** |
| Prettier, komanda sətrindən 6,305 Markdown faylı | 352 s | **22.5 s: 15.6x daha sürətli** |
| [json-server](real-world.md#json-server), bütöv bir Node serveri: boş vəziyyətdə yaddaş | 74 MB | **3.5 MB**, tək bir 5.3 MB binary |
| json-server, işə salınmadan ilk cavaba qədər | 79 ilə 85 ms arası | **8 ilə 10 ms arası** |
| 50,000 string üzərində [Fuse.js](real-world.md#fusejs) axtarışı, sorğu başına | 91.2 ms | **2.8 ms: 32x daha sürətli** |

Prettier: 17,224 qeydə alınmış icradan 15,861-si 10 dildə (JavaScript, TypeScript, JSX, JSON,
GraphQL, YAML, Markdown, CSS, SCSS, Less) tamamilə Rust-da formatlanır və 17,224-ün hamısı Prettier
ilə bayt-bayt uyğun gəlir, çünki hələ port edilməmiş hər şey Prettier-ə qayıdır. json-server: qeydə
alınmış 514/514 HTTP halı təmiz Rust binary-si üzərində uyğun gəlir. Apple M4-də, hər biri iki dəfə
işə salınaraq ölçülüb; hər çıxış vaxt ölçülməzdən əvvəl müqayisə edilir ([ətraflı](real-world.md)).
Hər kiçik funksiya üçün Rust-u ayrıca çağırmaq adətən JIT-dən *daha yavaşdır*; rust-anything
köçürməyə dəyən dövrü tapır və onu köçürür.

## Yalnız yavaş hissə

Bütün tətbiqi port etməyiniz lazım deyil. `go`-nu çox yavaş işləyən bir komandaya yönəldin, o da
vaxtı aparan tək funksiyanı port etsin:

```sh
rust-anything go --agent claude -- python3 app.py
```

İşə salınmanı profilləyir, köçürməyə dəyən ən xarici funksiyanı seçir (içindəki kiçik çağırışı yox,
dövrü; profil belə deyirsə, batch şəklində), onu yaradılmış girişlər və işə salınmanızın etdiyi
çağırışlarla qeydə alır və `src/port.rs` faylını agentinizə verir (`claude`, `codex` və ya promptu
stdin-dən oxuyan istənilən komanda; `--agent` olmadan sizin üçün `TASK.md` yazır və dayanır).
Funksiyanı yalnız qeydə alınmış hər hal uyğun gəldikdə və `gate` keçdikdə işə salır. Sonra:

```sh
rust-anything report rust/dedupe-rs     # pull request üçün REPORT.md və card.svg
rust-anything disable rust/dedupe-rs    # bayt-bayt orijinala qayıdış
```

Yuxarıdakı [Fuse.js](real-world.md#fusejs) sətri də bu formadadır: Fuse-un bir string üçün
hesabı bütün siyahı üzərində batch şəklində Rust-a köçürüldü; obyekt açarları, genişləndirilmiş
axtarış və qalan hər şey JavaScript-də qaldı. 50,000 string üzərində sorğu başına 91.2 ms-dən
2.8 ms-ə. `go`-nu 20 sətirlik Python dedupe olan
[`tests/fixtures/go-fuzzy`](../tests/fixtures/go-fuzzy) üzərində sınayın. Portu açıb-bağlamağın
bütün yolları: [Switch a port on and off](how-it-works.md#switch-a-port-on-and-off).

<p align="center">
  <img src="assets/json-server.gif" alt="Node serveri kimi və tək bir Rust binary-si kimi json-server: boş vəziyyətdə yaddaş 74 MB və 3.5 MB, işə salınmadan ilk cavaba qədər 79 ilə 85 ms və 8 ilə 10 ms arası, qeydə alınmış 514 HTTP halından 514-ü uyğun gəlir." width="820">
  <br>
  <sub>Bütöv bir Node serveri olan json-server təmiz Rust binary-sinə çevrildi (Apple M4). <a href="real-world.md#json-server">Necə ölçülüb</a>.</sub>
</p>

> rust-anything müstəqil layihədir, Rust Project və ya Rust Foundation ilə əlaqəli deyil və onlar
> tərəfindən dəstəklənmir. "Rust" Rust Foundation-ın əmtəə nişanıdır.

> [!WARNING]
> `record` və `app record` orijinal kodunuzu işə salır, hər funksiyanı (və ya bütöv tətbiqi) çoxlu
> yaradılmış girişlə çağırır, `app record` isə hər halı iki dəfə işlədir. `diff`, `gate` və `app replay`
> eyni halları port və ya orijinal üzərində yenidən işə salır. Fayl yazan, verilənlər bazası ilə
> danışan və ya şəbəkəyə müraciət edən kod bunu hər hal üçün bir dəfə edir. Onları layihənin
> surətində, test verilənlər bazası ilə və mühitdə production etimadnamələri olmadan işə salın.

## Sürətli başlanğıc

Bunu Claude Code, Codex, Cursor, Gemini CLI və ya OpenCode-a yapışdırın və quraşdırmanı agentinizə həvalə edin:

```text
rust-anything-i quraşdır: `cargo install rust-anything`, sonra `rust-anything setup` işə sal.
Səni nə vaxt yenidən başlatmalı olduğumu de.
```

Və ya özünüz:

```sh
cargo install rust-anything   # crates.io-dan
rust-anything setup           # Claude Code, Codex, Cursor, Gemini CLI və OpenCode-da qeydiyyatdan keçirir
```

Hələ Rust toolchain-iniz yoxdur? macOS, Linux və Windows üçün hazır binary fayllar hər
[GitHub release](https://github.com/erkamyaman/rust-anything/releases)-ə əlavə olunub, JavaScript
layihələri üçün isə `npm install -g rust-anything` eyni binary-ni quraşdırır.

Claude Code-da onu plugin kimi də quraşdıra bilərsiniz (MCP server və skill; yuxarıdakı CLI
`PATH`-da olmalıdır):

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

`setup` tapdığı hər agentə MCP serveri əlavə edir və iş axını skill-ini agentin skill-ləri oxuduğu
yerə quraşdırır. Digər ayarlarınızı saxlayır, yenidən işə salmaq təhlükəsizdir, `--remove` isə
etdiklərini geri qaytarır. Sonra agentinizdən, məsələn, bunları istəyin:

- *"Bu CLI-ı rust-anything ilə başdan-başa Rust-da yenidən yaz və eyni davrandığını sübut et."*
- *"Bu axtarış yavaş görünür. rust-anything ilə profilini çıxar və isti dövrü Rust-a köçür."*
- *"`src/parser/`-in rust-anything ilə Rust-a miqrasiyasını planlaşdır; ilk vahidi port et və sübut et."*
- *"`rust/text-rs`-dəki Rust portunu orijinala qarşı benchmark et və onu buraxmağa dəyib-dəymədiyini de."*

### Dəstəklənən agentlər

| Agent | `rust-anything setup` əlavə edir |
| --- | --- |
| Claude Code | `~/.claude.json`-da MCP server, `~/.claude/skills/`-də skill |
| Codex | `~/.codex/config.toml`-da MCP server, `~/.codex/skills/`-də skill |
| Cursor | `~/.cursor/mcp.json`-da MCP server |
| Gemini CLI | `~/.gemini/settings.json`-da MCP server |
| OpenCode | `~/.config/opencode/opencode.json`-da MCP server (`mcp.servers`), `~/.config/opencode/skills/`-də skill |
| İstənilən digər MCP müştərisi | `rust-anything mcp`-ni əllə stdio server kimi əlavə edin (bax [Quraşdırma](install.md#install)) |

Mövcud faylı dəyişməzdən əvvəl `setup` onun yanında bir surətini saxlayır (`<file>.rust-anything.bak`).

OpenCode-u əllə qoşmaq üçün serveri `~/.config/opencode/opencode.json`-a yazın (`command` bütöv
argv-dir) və [`skills/rust-anything/`](../skills/rust-anything) qovluğunu
`~/.config/opencode/skills/rust-anything/`-ə kopyalayın:

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

Və ya addımları özünüz icra edin:

```sh
rust-anything profile . -- node bench.mjs              # isti olan nədir və onu kim çağırır
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# src/port.rs-i yazın (və ya agentinizə yazdırın), sonra:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # hər qeydə alınmış hal uyğun gəlməlidir
rust-anything bench rust/fuzzy-rs                      # daha sürətlidir? hökm deyəcək
rust-anything gate rust/fuzzy-rs                       # hazırdır, yəni bu keçir
```

rust-anything kodu tərcümə etmir: Rust-u kodlaşdırma agentiniz (Claude Code, Codex, Cursor, OpenCode, ...) və ya
siz yazırsınız. rust-anything agentin etibarlı şəkildə təxmin edə bilmədiyini təmin edir: vaxtın hara
getdiyini, əvvəlcə nəyin və hansı ardıcıllıqla port ediləcəyini, kodun qalan hissəsinin Rust-u API-si
dəyişmədən çağıra bilməsi üçün körpünü (bridge), portun uyğun gəlməli olduğu dəqiq davranışı, onu
buraxmağın təhlükəsiz yolunu və portun nə vaxt hazır olduğunu deyən gate-i.

Eyni çıxış kifayət deyil. Tətbiqin göndərdiklərini də qeydə alın, onda e-poçtu iki dəfə göndərən yenidən yazılmış versiya uğursuz olur:

<p align="center">
  <img src="assets/effects.gif" alt="--effects smtp ilə app record hər qeydiyyat üçün bir salamlama e-poçtunu qeydə alır; eyni çıxışı çap edən, lakin e-poçtu iki dəfə göndərən yenidən yazılmış versiyanın app replay-i 0/2 halın keçdiyini, effektlərdə uğursuz olduğunu bildirir və hesabat artıq e-poçtu göstərir." width="820">
  <br>
  <sub><code>app record --effects</code> və <code>app replay</code>-in real icrası. Bax: <a href="whole-apps.md#side-effects-email-webhooks-and-databases">Yan təsirlər</a>.</sub>
</p>

## Dillər

| Dil | Vahid | Körpü | Qeydə alan və təkrar oynadan |
| --- | --- | --- | --- |
| TypeScript/JavaScript | qovluq və ya fayl | yaradılmış binary wire ilə napi-rs və ya wasm-bindgen; siniflər və async daxil | `.ts` mənbələrini işlədən Node |
| Python | paket və ya modul | PyO3 (abi3), birbaşa çevirmə, tipli exception-lar | orijinalı import edən Python |
| C | `.c` faylı | orijinal adlarla `extern "C"` funksiyaları və header | paylaşılan kitabxana kimi kompilyasiya olunmuş orijinal, ctypes ilə çağırılır, hər hal üçün bir fork |
| Go | paket | orijinal API-li paketdə tipli cgo çağırışları və yaradılmış binary wire | öz modulu kimi qurulan yaradılmış Go drayveri |
| Ruby | modul funksiyalarından ibarət fayl və ya qovluq | orijinal modulların altında magnus genişlənməsi | yaradılmış Ruby drayveri |
| Java | `public static` metodlardan ibarət paket | orijinal metodlara malik wrapper siniflərdə tipli JNI və yaradılmış binary wire | yaradılmış reflection drayveri |
| PHP | funksiyalardan və `public static` metodlardan ibarət qovluq | növbədə: C ABI üzərindən PHP FFI | bu gün bütöv tətbiqlər: CLI-lar və serverlər (`php -S`) üçün `app record`, `app replay` və front door |
| C# | `public static` metodlardan ibarət qovluq | növbədə: C ABI üzərindən P/Invoke | bu gün bütöv tətbiqlər: CLI-lar və ASP.NET serverləri (`dotnet <app>.dll`) üçün `app record`, `app replay` və front door |

C-dən başqa hər dil batch sinifləri də alır (`--batch`): elementlər körpünü bir dəfə keçir və
Rust-da qalır, hər çağırış isə bütün nüvələrdə hamısının üzərində işləyir.

Dil repozitoriyadan müəyyən edilir (`go.mod`, `pom.xml`, `.csproj` və ya `.sln`, `pyproject.toml`,
`Gemfile`, `composer.json`, `package.json`, əks halda ən çox rast gəlinən mənbə faylları); `--lang`
bunu əvəz edir. Tiplər mənbədən gəlir: TS tipləri, Python type hint-ləri, C, Go, Java və C#
imzaları, Ruby YARD teqləri, PHP tipləri və phpdoc; rəqəm kimi istifadə olunan tipsiz parametrlər
rəqəm kimi müəyyən edilir.

PHP üçün `analyze` PHP tətbiqini planlaşdırır (Composer paketləri və PSR-4 autoloading,
sərbəst müqayisə və tam ədədin float-a daşması kimi risklər, crate-lərə uyğunlaşdırılmış Composer
paketləri), PHP CLI və ya serveri isə qeydə alına, təkrar oynadıla və Rust front door-unun arxasına
qoyula bilər. Onun vahidlərini PHP-nin FFI-ı vasitəsilə Rust-a port etmək növbəti addımdır
([issue #2](https://github.com/erkamyaman/rust-anything/issues/2)).

C# da eyni şəkildə işləyir: `analyze` `.csproj` layihələrini və onların NuGet paketlərini
oxuyur (crate-lərə uyğunlaşdırılmış), `using` direktivlərini qovluqlara həll edir və C#-ın fərqli
etdiyi şeyləri işarələyir (`decimal`, `dynamic`, LINQ-in təxirə salınmış sorğuları, `Task`, `lock`,
reflection). C# CLI və ya ASP.NET serveri qeydə alına, təkrar oynadıla və Rust front door-unun
arxasına qoyula bilər; vahidləri P/Invoke vasitəsilə port etmək növbədədir.

## Ətraflı

- [Necə işləyir](how-it-works.md): dövr, `bridge` və `analyze` nəyə qərar verir, hot path-lar, nəyi tuta bilmir, real portun nə tələb etdiyi və orijinalla ayaqlaşmaq (`map`)
- [Bütöv tətbiqi port etmək](whole-apps.md): serverlər, CLI-lar və kitabxanalar, front door-dan `app lift`-ə qədər
- [Nümunələr və nümunə araşdırmaları](examples.md): x18n və altı dildə real kod
- [Real portlar](real-world.md): json-server, Fuse.js, vercel/ms, benchmark-larla
- [Quraşdırma, yeniləmə və inkişaf](install.md): bütün quraşdırma üsulları, Windows və mənbədən qurma
- [Tez-tez verilən suallar](faq.md) · [Vəziyyət](STATUS.md) · [MCP alətləri](tools.md)

## Sponsorlar

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> pulsuz və açıq mənbəlidir. Əgər işinizə yarayırsa, lütfən ona sponsorluq etməyi düşünün.<br>
  Dəstəyiniz inkişafın davam etməsinə kömək edir.
</p>

<p align="center">
  <a href="https://coderabbit.link/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="Şirkət sponsoru: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="Sponsorlar: Santosh Yadav, Hakan Soytürk, Ali Yilmaz, Ashley Hunter, Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/Sponsor_olun-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="Sponsor olun"></a>
  <a href="https://buymeacoffee.com/erkamyaman"><img src="https://img.shields.io/badge/Buy_me_a_coffee-%E2%98%95-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee"></a>
</p>

## Töhfə vermək

Issue-lar və pull request-lər xoş qarşılanır; [CONTRIBUTING.md](../CONTRIBUTING.md) quraşdırmanı,
testlərin harada yerləşdiyini və yeni dilin necə əlavə edildiyini izah edir. Dizayn qərarları və
onların səbəbləri [DECISIONS.md](../DECISIONS.md)-dədir. Real portların nümunə araşdırmaları xüsusilə
xoş qarşılanır. Təhlükəsizlik problemlərini lütfən gizli bildirin ([SECURITY.md](../SECURITY.md)) və
[davranış kodeksinə](../CODE_OF_CONDUCT.md) baxın.

## Lisenziya

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). Repozitoriyanızdakı portlar və yaradılmış kod sizindir.
