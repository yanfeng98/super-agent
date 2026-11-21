<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="rust-anything крабы">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><b>Кушымтагызны Rust телендә яңадан языгыз һәм аның элеккечә эшләвен исбатлагыз.</b></p>

<p align="center">2.0 версиясендә яңа: <a href="real-world.md#prettier">Prettier, Rust'ка күчерелде</a>. 19x тизрәк · язып алынган 17,224 җибәрүнең 15,837се тулысынча Rust'та · барысы да байтка байт туры килә</p>

<p align="center">
  <a href="https://crates.io/crates/rust-anything"><img src="https://img.shields.io/crates/v/rust-anything?color=E2862A&label=crates.io" alt="crates.io версиясе"></a>
  <a href="https://www.npmjs.com/package/rust-anything"><img src="https://img.shields.io/npm/v/rust-anything?color=E2862A&label=npm" alt="npm версиясе"></a>
  <a href="https://github.com/erkamyaman/rust-anything/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/erkamyaman/rust-anything/ci.yml?branch=main&label=tests" alt="Тестлар"></a>
  <a href="https://github.com/erkamyaman/rust-anything"><img src="https://img.shields.io/github/stars/erkamyaman/rust-anything?style=social" alt="GitHub йолдызлары"></a>
  <a href="https://github.com/AIshihzong/awesome-reverse-engineer-anything"><img src="https://img.shields.io/badge/Featured%20in-Awesome%20REA-f0b429" alt="Featured in Awesome REA"></a>
</p>

<p align="center">
  <a href="#тиз-башлау">Тиз башлау</a> ·
  <a href="#телләр">Телләр</a> ·
  <a href="real-world.md">Чын портлар</a> ·
  <a href="how-it-works.md#how-it-works">Ничек эшли</a> ·
  <a href="whole-apps.md#port-a-whole-app">Бөтен кушымталар</a> ·
  <a href="faq.md#faq">Еш бирелә торган сораулар</a>
</p>

<p align="center">Төркиядә 🇹🇷 <a href="https://github.com/erkamyaman">erKam Yaman</a> ясаган · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ Миңа спонсор булыгыз</a></p>

<p align="center">Спонсор: <a href="https://coderabbit.ai"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · Татарча · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a></sub></p>
<p align="center"><sub>Бу тәрҗемәне җәмәгатьчелек алып бара, шуңа күрә ул <a href="../README.md">инглизчә README</a> файлыннан калышырга мөмкин.</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>Язып алу</b><br>
      <code>record</code> оригиналны ясалган чик очракларда, семантик тозакларда һәм сезнең үз кертмәләрегездә җибәрә һәм ул нәрсә кайтарганын яки нинди хата ыргытканын саклый.
    </td>
    <td width="25%" valign="top">
      <b>Күчерү</b><br>
      <code>bridge</code> Rust crate'ын һәм wrapper ясый, шуңа күрә чакыручылар оригинал API'ны саклый. Rust кодын сез яки агентыгыз яза.
    </td>
    <td width="25%" valign="top">
      <b>Тикшерү</b><br>
      <code>diff</code> язып алынган һәр очракны Rust'та кабат уйната, һәм алар барысы да туры килмичә һәм <code>gate</code> узмыйча, бернәрсә дә чыгарылмый.
    </td>
    <td width="25%" valign="top">
      <b>Lift</b><br>
      Кабат уйнату fallback'сыз узса һәм һәр берәмлек gate'ны узса, <code>app lift</code> аны саф Rust кушымтасына әйләндерә. Оригиналга беркайчан да кагылмый.
    </td>
  </tr>
</table>

**Бөтен кушымтагызны Rust телендә өлешләп яңадан языгыз.** rust-anything сезнең Node.js (TypeScript/JavaScript),
Python, C, Go, Ruby яки Java кушымтагыз бүген нәрсә эшләгәнен язып ала, аннары Rust телендәге
яңа версиянең һәр өлешен, алга барганчы, шул язма белән чагыштырып тикшерә, кушымта саф Rust булганчы. CLI, HTTP
серверлар һәм китапханәләр; Rust кодын сезнең программалау агенты (яки сез үзегез) яза. Ул язма
нәрсәне колачлаганын гына тотып кала ала: [Ул нәрсәне тотып кала алмый](how-it-works.md#what-it-cant-catch) бүлеген карагыз. Бөтен кораллар һәм серверлар, яки әкрен өлеш кенә,
бер тапкырда бер кайнар юл (hot path):

| Чын проект, нәтиҗә оригиналдагы кебек | Оригинал | rust-anything белән |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2, командалар юлыннан 2,360 JS/TS файл | 128.6 s | **6.9 s: 19x тизрәк** |
| Prettier, командалар юлыннан 6,305 Markdown файл | 352 s | **22.5 s: 15.6x тизрәк** |
| [json-server](real-world.md#json-server), бөтен бер Node сервер: буш торганда хәтер | 74 MB | **3.5 MB**, бер 5.3 MB бинар файл |
| json-server, җибәрүдән беренче җавапка кадәр | 79 белән 85 ms арасы | **8 белән 10 ms арасы** |
| [Fuse.js](real-world.md#fusejs) белән 50,000 юл буенча эзләү, бер сорауга | 91.2 ms | **2.8 ms: 32x тизрәк** |

Prettier: язып алынган 17,224 җибәрүдән 15,837 җибәрү 10 телдә (JavaScript, TypeScript, JSX, JSON,
GraphQL, YAML, Markdown, CSS, SCSS, Less) тулысынча Rust'та форматлана, һәм бөтен 17,224 җибәрү дә
Prettier белән байтка-байт туры килә, чөнки әле күчерелмәгән һәрнәрсә Prettier'га кире кайта (fallback).
json-server: язып алынган 514/514 HTTP очрагы саф Rust бинар файлында туры килә. Apple M4'та үлчәнде,
һәрберсе ике тапкыр җибәрелде, һәр нәтиҗә вакыт үлчәнгәнче чагыштырылды ([тулырак](real-world.md)).
Һәр кечкенә функция өчен Rust'ны аерым чакыру гадәттә JIT'тан *әкренрәк*; rust-anything күчерергә
лаеклы циклны таба һәм аны күчерә.

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier һәм аның Rust порты шул ук 59 чын файлны форматлый, һәр файлга бер процесс: Prettier 3.63 s, Rust порты 0.20 s сарыф итә, нәтиҗә байтка-байт бертөрле." width="820">
  <br>
  <sub>ms, Fuse.js һәм json-server проектларыннан алынган 59 чын файлда Prettier һәм аның Rust порты (Apple M4). Нәтиҗә байтка-байт бертөрле. <a href="real-world.md#prettier">Ничек үлчәнде</a>.</sub>
</p>

<p align="center">
  <img src="assets/json-server.gif" alt="json-server Node сервер буларак һәм бер Rust бинар файлы буларак: буш торганда хәтер 74 MB һәм 3.5 MB, җибәрүдән беренче җавапка кадәр 79 белән 85 ms һәм 8 белән 10 ms арасы, язып алынган 514 HTTP очракның 514е туры килә." width="820">
  <br>
  <sub>json-server, бөтен бер Node сервер, саф Rust бинар файлына әйләндерелде (Apple M4). <a href="real-world.md#json-server">Ничек үлчәнде</a>.</sub>
</p>

> rust-anything бәйсез проект, ул Rust Project яки Rust Foundation белән бәйле түгел һәм алар тарафыннан хупланмаган.
> "Rust" Rust Foundation'ның сәүдә билгесе.

> [!WARNING]
> `record` һәм `app record` сезнең оригинал кодны җибәрә: һәр функцияне (яки бөтен кушымтаны) күп санлы
> ясалган кертмәләр белән чакыра, ә `app record` һәр очракны ике тапкыр җибәрә. `diff`, `gate` һәм `app replay`
> шул ук очракларны портта яки оригиналда тагын җибәрә. Файл язучы, мәгълүмат базасы белән эшләүче яки
> челтәргә мөрәҗәгать итүче код моны һәр очрак саен бер тапкыр эшли. Аларны проектның күчермәсендә,
> тест базасы белән һәм мохиттә production сертификатлары булмаганда җибәрегез.

## Тиз башлау

Моны Claude Code, Codex, Cursor, Gemini CLI яки OpenCode эченә куегыз һәм агентыгыз аны урнаштырсын:

```text
Install rust-anything: run `cargo install rust-anything`, then `rust-anything setup`.
Tell me when to restart you.
```

Яки үзегез:

```sh
cargo install rust-anything   # crates.io'дан
rust-anything setup           # аны Claude Code, Codex, Cursor, Gemini CLI һәм OpenCode'та теркәп куя
```

Әле Rust toolchain юкмы? macOS, Linux һәм Windows өчен әзер бинар файллар һәр
[GitHub релизына](https://github.com/erkamyaman/rust-anything/releases) беркетелгән, ә JavaScript
проектлары өчен `npm install -g rust-anything` шул ук бинар файлны урнаштыра.

Claude Code'та аны плагин итеп тә урнаштырып була (MCP сервер һәм skill; өстәге CLI
сезнең `PATH` эчендә булырга тиеш):

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

`setup` тапкан һәр агентка MCP серверны өсти һәм workflow skill'ны агент skill'ларны укыган урынга
урнаштыра. Ул сезнең башка көйләнмәләрегезне саклый, аны кабат җибәрү куркынычсыз, ә `--remove` аны кире кайтара.
Аннары агентыгыздан сорагыз, мәсәлән:

- *"Бу CLI'ны rust-anything ярдәмендә башыннан ахырына кадәр Rust телендә яңадан яз һәм аның шулай ук эшләвен исбатла."*
- *"Бу эзләү әкрен кебек. Аны rust-anything белән профильлә һәм кайнар циклны Rust'ка күчер."*
- *"rust-anything белән `src/parser/` папкасын Rust'ка күчерүне планлаштыр; беренче берәмлекне күчер һәм исбатла."*
- *"`rust/text-rs` эчендәге Rust портын оригинал белән бенчмаркта чагыштыр һәм аны чыгарыргамы икәнен әйт."*

### Кайсы агентлар белән эшли

| Агент | `rust-anything setup` нәрсә өсти |
| --- | --- |
| Claude Code | `~/.claude.json` эчендә MCP сервер, `~/.claude/skills/` эчендә skill |
| Codex | `~/.codex/config.toml` эчендә MCP сервер, `~/.codex/skills/` эчендә skill |
| Cursor | `~/.cursor/mcp.json` эчендә MCP сервер |
| Gemini CLI | `~/.gemini/settings.json` эчендә MCP сервер |
| OpenCode | `~/.config/opencode/opencode.json` эчендә MCP сервер (`mcp.servers`), `~/.config/opencode/skills/` эчендә skill |
| Башка теләсә нинди MCP клиент | `rust-anything mcp` командасын stdio сервер итеп кулдан өстәгез ([Урнаштыру](install.md#install) бүлеген карагыз) |

Булган файлны үзгәртер алдыннан `setup` аның күчермәсен янәшәдә саклый (`<file>.rust-anything.bak`).

OpenCode'ны кулдан тоташтыру өчен серверны `~/.config/opencode/opencode.json` эченә куегыз
(`command` бөтен argv була) һәм [`skills/rust-anything/`](../skills/rust-anything) папкасын
`~/.config/opencode/skills/rust-anything/` эченә күчерегез:

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

Яки адымнарны үзегез башкарыгыз:

```sh
rust-anything profile . -- node bench.mjs              # нәрсә кайнар һәм аны кем чакыра
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# src/port.rs языгыз (яки агентка тапшырыгыз), аннары:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # язып алынган һәр очрак туры килергә тиеш
rust-anything bench rust/fuzzy-rs                      # тизрәкме? нәтиҗә әйтә
rust-anything gate rust/fuzzy-rs                       # бу узса, эш бетте
```

rust-anything кодны тәрҗемә итми: Rust кодын сезнең программалау агенты (Claude Code, Codex, Cursor, OpenCode, ...) яки сез
үзегез язасыз. rust-anything агент ышанычлы итеп чамалый алмаган нәрсәләрне бирә: вакыт кая китә, нәрсәне
беренче һәм нинди тәртиптә күчерергә, калган код Rust'ны API үзгәрмичә чакырсын өчен bridge,
порт туры килергә тиешле төгәл тәртип, аны чыгаруның куркынычсыз ысулы һәм порт кайчан
әзер икәнен әйтүче gate.

Бертөрле нәтиҗә генә җитми. Кушымта тышка нәрсә җибәргәнен дә язып алыгыз, шунда хатны ике тапкыр
җибәрүче яңа версия узмый:

<p align="center">
  <img src="assets/effects.gif" alt="--effects smtp белән app record һәр теркәлү өчен бер сәламләү хатын язып ала; шул ук нәтиҗәне чыгаручы, ләкин хатны ике тапкыр җибәрүче яңа версиядә app replay 0/2 очрак узганын һәм effects буенча уңышсыз булганын хәбәр итә, ә хисап артык хатны күрсәтә." width="820">
  <br>
  <sub><code>app record --effects</code> һәм <code>app replay</code> командаларының чын җибәрелеше. <a href="whole-apps.md#side-effects-email-webhooks-and-databases">Ян йогынтылар</a> бүлеген карагыз.</sub>
</p>

## Телләр

| Тел | Берәмлек | Bridge | Кем язып ала һәм кабат уйната |
| --- | --- | --- | --- |
| TypeScript/JavaScript | папка яки файл | ясалган бинар wire форматлы napi-rs яки wasm-bindgen; класслар һәм async та керә | Node, `.ts` чыганакларын җибәреп |
| Python | пакет яки модуль | PyO3 (abi3), туры үзгәртү, типлы исключениеләр | Python, оригиналны импортлап |
| C | `.c` файл | оригинал исемле `extern "C"` функцияләр һәм header | уртак китапханәгә компиляцияләнгән оригинал, ctypes аша чакырыла, һәр очракка бер fork |
| Go | пакет | типлы cgo чакырулары һәм ясалган бинар wire формат, оригинал API'лы пакетта | ясалган Go драйверы, аерым модуль итеп җыела |
| Ruby | модуль функцияләре булган файл яки папка | оригинал модульләр астында magnus киңәйтмәсе | ясалган Ruby драйверы |
| Java | `public static` методлар пакеты | типлы JNI һәм ясалган бинар wire формат, оригинал методлы wrapper класслар эчендә | ясалган reflection драйверы |
| PHP | функцияләр һәм `public static` методлар папкасы | киләсе адым: C ABI өстендә PHP FFI | бүген үк бөтен кушымталар: `app record`, `app replay` һәм front door, CLI һәм серверлар өчен (`php -S`) |
| C# | `public static` методлар папкасы | киләсе адым: C ABI өстендә P/Invoke | бүген үк бөтен кушымталар: `app record`, `app replay` һәм front door, CLI һәм ASP.NET серверлар өчен (`dotnet <app>.dll`) |

C'тан кала һәр тел batch класслар да ала (`--batch`): элементлар бер тапкыр күчә һәм
Rust'та кала, ә һәр чакыру аларның барысын да барлык ядроларда эшкәртә.

Тел репозиторийдан билгеләнә (`go.mod`, `pom.xml`, `.csproj` яки `.sln`,
`pyproject.toml`, `Gemfile`, `composer.json`, `package.json`, юкса иң күп очрый торган чыганак файллар);
`--lang` аны алыштыра. Типлар чыганак кодтан алына: TS типлары, Python type hints, C, Go, Java һәм C#
сигнатуралары, Ruby YARD теглары, PHP типлары һәм phpdoc; сан итеп кулланылган типсыз параметрлар
сан дип чыгарыла.

PHP өчен `analyze` PHP кушымтасын планлаштыра (Composer пакетлары һәм PSR-4 autoloading, катгый булмаган
чагыштыру һәм бөтен саннарның float'ка ташып чыгуы кебек куркынычлар, Composer пакетларына туры килгән crate'лар),
ә PHP CLI яки серверны язып алып, кабат уйнатып, Rust front door артына куеп була. Аның
берәмлекләрен PHP FFI аша Rust'ка күчерү киләсе адым ([issue #2](https://github.com/erkamyaman/rust-anything/issues/2)).

C# та шулай ук эшли: `analyze` `.csproj` проектларын һәм аларның NuGet пакетларын укый
(crate'ларга туры китереп), `using` директиваларын папкаларга бәйли һәм C# нәрсәне башкача эшләгәнен билгели
(`decimal`, `dynamic`, LINQ'ның кичектерелгән сораулары, `Task`, `lock`, reflection). C# CLI яки ASP.NET
серверын язып алып, кабат уйнатып, Rust front door артына куеп була; берәмлекләрне P/Invoke аша күчерү
киләсе адым.

## Күбрәк белү

- [Ничек эшли](how-it-works.md): цикл, `bridge` һәм `analyze` нәрсәне хәл итә, кайнар юллар, ул нәрсәне тотып кала алмый, чын порт нәрсә таләп итә һәм оригиналдан калышмау (`map`)
- [Бөтен кушымтаны күчерү](whole-apps.md): серверлар, CLI һәм китапханәләр, front door'дан `app lift`'ка кадәр
- [Мисаллар һәм тикшеренүләр](examples.md): x18n һәм алты телдәге чын код
- [Чын портлар](real-world.md): json-server, Fuse.js, vercel/ms, бенчмарклар белән
- [Урнаштыру, яңарту һәм эшләнмә](install.md): урнаштыруның һәр ысулы, Windows һәм чыганак коддан җыю
- [Еш бирелә торган сораулар](faq.md) · [Халәт](STATUS.md) · [MCP кораллары](tools.md)

## Спонсорлар

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> бушлай һәм ачык чыганаклы. Әгәр ул эшегездә ярдәм итсә, зинһар, аңа спонсорлык итүне уйлап карагыз.<br>
  Сезнең ярдәм эшләнмәне дәвам итәргә мөмкинлек бирә.
</p>

<p align="center">
  <a href="https://coderabbit.ai">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="Компания спонсоры: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="Спонсорлар: Santosh Yadav, Hakan Soytürk, Ali Yilmaz, Ashley Hunter, Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/%D0%A1%D0%BF%D0%BE%D0%BD%D1%81%D0%BE%D1%80_%D0%B1%D1%83%D0%BB%D1%83-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="Спонсор булу"></a>
</p>

## Катнашу

Issue'лар һәм pull request'лар хуплана; [CONTRIBUTING.md](../CONTRIBUTING.md) көйләүне, тестларны
кая куярга һәм яңа телне ничек өстәргә икәнен аңлата. Проект карарлары һәм аларның сәбәпләре
[DECISIONS.md](../DECISIONS.md) эчендә. Чын портлар турында мисаллар аеруча хуплана. Зинһар, куркынычсызлык
проблемалары турында шәхси рәвештә хәбәр итегез ([SECURITY.md](../SECURITY.md)) һәм
[тәртип кодексы](../CODE_OF_CONDUCT.md) белән танышыгыз.

## Лицензия

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). Репозиторийыгыздагы портлар һәм ясалган код сезнеке.
