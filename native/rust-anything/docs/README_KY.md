<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="rust-anything краб">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><b>Колдонмоңузду Rust тилинде кайра жазып, анын мурункудай иштээрин далилдеңиз.</b></p>

<p align="center">2.0 версиясындагы жаңылык: <a href="real-world.md#prettier">Prettier, Rust'ка порттолду</a>. 19x ылдамыраак · 17,224 жазылган иштетүүнүн 15,837си толугу менен Rust'та · баары байтка чейин бирдей</p>

<p align="center">
  <a href="https://crates.io/crates/rust-anything"><img src="https://img.shields.io/crates/v/rust-anything?color=E2862A&label=crates.io" alt="crates.io версиясы"></a>
  <a href="https://www.npmjs.com/package/rust-anything"><img src="https://img.shields.io/npm/v/rust-anything?color=E2862A&label=npm" alt="npm версиясы"></a>
  <a href="https://github.com/erkamyaman/rust-anything/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/erkamyaman/rust-anything/ci.yml?branch=main&label=tests" alt="Тесттер"></a>
  <a href="https://github.com/erkamyaman/rust-anything"><img src="https://img.shields.io/github/stars/erkamyaman/rust-anything?style=social" alt="GitHub жылдыздары"></a>
  <a href="https://github.com/AIshihzong/awesome-reverse-engineer-anything"><img src="https://img.shields.io/badge/Featured%20in-Awesome%20REA-f0b429" alt="Featured in Awesome REA"></a>
</p>

<p align="center">
  <a href="#тез-баштоо">Тез баштоо</a> ·
  <a href="#тилдер">Тилдер</a> ·
  <a href="real-world.md">Чыныгы порттор</a> ·
  <a href="how-it-works.md#how-it-works">Кантип иштейт</a> ·
  <a href="whole-apps.md#port-a-whole-app">Бүт колдонмолор</a> ·
  <a href="faq.md#faq">Көп берилүүчү суроолор</a>
</p>

<p align="center">Түркияда 🇹🇷 <a href="https://github.com/erkamyaman">erKam Yaman</a> жасаган · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ Мага демөөрчү болуңуз</a></p>

<p align="center">Демөөрчү: <a href="https://coderabbit.ai"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · Кыргызча · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a></sub></p>
<p align="center"><sub>Бул котормону коомчулук колдойт, ошондуктан ал <a href="../README.md">англисче README</a> нускасынан артта калышы мүмкүн.</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>Жазып алуу</b><br>
      <code>record</code> түп нусканы генерацияланган чектик кейстерде, семантикалык тузактарда жана өзүңүздүн киргизүүлөрүңүздө иштетип, ал эмне кайтарарын же кандай ката ыргытарын сактайт.
    </td>
    <td width="25%" valign="top">
      <b>Порттоо</b><br>
      <code>bridge</code> Rust crate'ин жана wrapper'ди генерациялайт, ошентип чакыруучулар түп нуска API'син сакташат. Rust кодун сиз же агентиңиз жазат.
    </td>
    <td width="25%" valign="top">
      <b>Текшерүү</b><br>
      <code>diff</code> ар бир жазылган кейсти Rust'ка каршы кайра ойнотот, алардын баары дал келмейинче жана <code>gate</code> өтмөйүнчө эч нерсе чыгарылбайт.
    </td>
    <td width="25%" valign="top">
      <b>Lift</b><br>
      Replay fallback'сиз өтүп, ар бир бирдик gate'тен өткөндө, <code>app lift</code> аны таза Rust колдонмосуна айлантат. Түп нускага эч качан тийбейт.
    </td>
  </tr>
</table>

**Бүт колдонмоңузду Rust тилинде бөлүк-бөлүгү менен кайра жазыңыз.** rust-anything сиздин Node.js
(TypeScript/JavaScript), Python, C, Go, Ruby же Java колдонмоңуз бүгүн эмне кылып жатканын жазып
алат, андан кийин Rust нускасынын ар бир бөлүгүн кийинкисине өтүүдөн мурун ошол жазуу менен салыштырып
текшерет, колдонмо толугу менен таза Rust болгонго чейин. CLI'лар, HTTP серверлер жана китепканалар;
Rust кодун сиздин коддоо агентиңиз (же өзүңүз) жазат. Ал жазуу камтыган нерсени гана кармай алат:
[Ал эмнени кармай албайт](how-it-works.md#what-it-cant-catch) бөлүмүн караңыз. Бүт куралдар жана
серверлер, же жай бөлүгү гана, бир убакта бир hot path:

| Чыныгы долбоор, чыгышы түп нуска менен бирдей | Түп нуска | rust-anything менен |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2, командалык саптан 2,360 JS/TS файл | 128.6 s | **6.9 s: 19x ылдамыраак** |
| Prettier, командалык саптан 6,305 Markdown файл | 352 s | **22.5 s: 15.6x ылдамыраак** |
| [json-server](real-world.md#json-server), бүт Node сервер: бош турганда эс тутум | 74 MB | **3.5 MB**, бир 5.3 MB бинардык файл |
| json-server, иштетүүдөн биринчи жоопко чейин | 79 менен 85 ms аралыгы | **8 менен 10 ms аралыгы** |
| [Fuse.js](real-world.md#fusejs) менен 50,000 сап боюнча издөө, бир суроого | 91.2 ms | **2.8 ms: 32x ылдамыраак** |

Prettier: 17,224 жазылган иштетүүнүн ичинен 15,837 иштетүү 10 тилде (JavaScript, TypeScript, JSX,
JSON, GraphQL, YAML, Markdown, CSS, SCSS, Less) толугу менен Rust ичинде форматталат, ал эми бардык
17,224 иштетүү Prettier менен байтма-байт дал келет, анткени азырынча порттолбогон ар бир нерсе
Prettier'ге кайтарылат (fallback). json-server: жазылган 514/514 HTTP кейс таза Rust бинардык
файлында дал келет. Apple M4'тө өлчөндү, ар бири эки жолу иштетилди, ар бир чыгыш убакыт өлчөнгөнгө
чейин салыштырылды ([кеңири](real-world.md)). Ар бир кичинекей функция үчүн Rust'ту өзүнчө чакыруу
адатта JIT'тен *жайыраак*; rust-anything көчүрүүгө арзый турган циклди таап, ошону көчүрөт.

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier жана анын Rust порту ошол эле 59 чыныгы файлды форматтайт, ар бир файлга бир процесс: Prettier 3.63 s, Rust порту 0.20 s коротот, чыгышы байтма-байт бирдей." width="820">
  <br>
  <sub>ms, Fuse.js жана json-server долбоорлорунан алынган 59 чыныгы файлда Prettier жана анын Rust порту (Apple M4). Чыгышы байтма-байт бирдей. <a href="real-world.md#prettier">Кантип өлчөнгөн</a>.</sub>
</p>

<p align="center">
  <img src="assets/json-server.gif" alt="json-server Node сервер катары жана бир Rust бинардык файлы катары: бош турганда эс тутум 74 MB жана 3.5 MB, иштетүүдөн биринчи жоопко чейин 79 менен 85 ms жана 8 менен 10 ms аралыгы, жазылган 514 HTTP кейстин 514ү дал келет." width="820">
  <br>
  <sub>json-server, бүт Node сервер, таза Rust бинардык файлына айлантылды (Apple M4). <a href="real-world.md#json-server">Кантип өлчөнгөн</a>.</sub>
</p>

> rust-anything көз карандысыз долбоор, ал Rust Project же Rust Foundation уюмдары менен байланышкан
> эмес жана алар тарабынан колдоого алынган эмес. "Rust" Rust Foundation уюмунун соода белгиси.

> [!WARNING]
> `record` жана `app record` түп нуска кодуңузду иштетет: ар бир функцияны (же бүт колдонмону) көптөгөн
> генерацияланган киргизүүлөр менен чакырат, ал эми `app record` ар бир кейсти эки жолу иштетет. `diff`,
> `gate` жана `app replay` ошол эле кейстерди портто же түп нускада кайрадан иштетет. Файл жазган,
> маалымат базасы менен иштеген же тармакка кайрылган код муну ар бир кейс үчүн бир жолу аткарат. Аларды
> долбоордун көчүрмөсүндө, тесттик маалымат базасы менен жана чөйрөдө production эсептик маалыматтары жок
> абалда иштетиңиз.

## Тез баштоо

Муну Claude Code, Codex, Cursor, Gemini CLI же OpenCode ичине чаптап, орнотууну агентиңизге тапшырыңыз:

```text
Install rust-anything: run `cargo install rust-anything`, then `rust-anything setup`.
Tell me when to restart you.
```

Же өзүңүз:

```sh
cargo install rust-anything   # from crates.io
rust-anything setup           # registers it with Claude Code, Codex, Cursor, Gemini CLI and OpenCode
```

Rust toolchain азырынча жокпу? macOS, Linux жана Windows үчүн даяр бинардык файлдар ар бир
[GitHub release](https://github.com/erkamyaman/rust-anything/releases) барагына тиркелген, ал эми
JavaScript долбоорлору үчүн `npm install -g rust-anything` ошол эле бинардык файлды орнотот.

Claude Code ичинде аны плагин катары да орнотсоңуз болот (MCP сервер жана skill; жогорудагы CLI сиздин
`PATH` ичинде болушу керек):

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

`setup` тапкан ар бир агентке MCP серверди кошот жана workflow skill'ин агент skill'дерди окуган жерге
орнотот. Ал башка жөндөөлөрүңүздү сактайт, аны кайра иштетүү коопсуз, ал эми `--remove` баарын артка
кайтарат. Андан кийин агентиңизден, мисалы, мындай сураңыз:

- *"Бул CLI'ды rust-anything менен башынан аягына чейин Rust тилинде кайра жаз жана анын так ошондой иштээрин далилде."*
- *"Бул издөө жай иштеп жаткандай. Аны rust-anything менен профилде жана hot loop'ту Rust'ка көчүр."*
- *"rust-anything менен `src/parser/` папкасын Rust'ка көчүрүү планын түз; биринчи бирдикти порттоп, далилде."*
- *"`rust/text-rs` ичиндеги Rust портун түп нуска менен бенчмарк аркылуу салыштыр да, аны чыгаруу керекпи, айт."*

### Шайкеш агенттер

| Агент | `rust-anything setup` кошкону |
| --- | --- |
| Claude Code | `~/.claude.json` ичинде MCP сервер, `~/.claude/skills/` ичинде skill |
| Codex | `~/.codex/config.toml` ичинде MCP сервер, `~/.codex/skills/` ичинде skill |
| Cursor | `~/.cursor/mcp.json` ичинде MCP сервер |
| Gemini CLI | `~/.gemini/settings.json` ичинде MCP сервер |
| OpenCode | `~/.config/opencode/opencode.json` ичинде MCP сервер (`mcp.servers`), `~/.config/opencode/skills/` ичинде skill |
| Башка каалаган MCP кардар | `rust-anything mcp` буйругун stdio сервер катары кол менен кошуңуз ([Орнотуу](install.md#install) бөлүмүн караңыз) |

Бар файлды өзгөртүүдөн мурун `setup` анын көчүрмөсүн жанына сактайт (`<file>.rust-anything.bak`).

OpenCode'ду кол менен туташтыруу үчүн серверди `~/.config/opencode/opencode.json` ичине коюңуз
(`command` толук argv болуп саналат) жана [`skills/rust-anything/`](../skills/rust-anything) папкасын
`~/.config/opencode/skills/rust-anything/` ичине көчүрүңүз:

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

Же кадамдарды өзүңүз аткарыңыз:

```sh
rust-anything profile . -- node bench.mjs              # what is hot, and who calls it
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# write src/port.rs (or let your agent), then:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # every recorded case must match
rust-anything bench rust/fuzzy-rs                      # faster? the verdict says
rust-anything gate rust/fuzzy-rs                       # done means this passes
```

rust-anything кодду которбойт: Rust кодун сиздин коддоо агентиңиз (Claude Code, Codex, Cursor, OpenCode, ...)
же өзүңүз жазасыз. rust-anything агент ишенимдүү түрдө болжолдой албаган нерселерди берет: убакыт
кайда кетет, адегенде эмнени жана кандай тартипте порттоо керек, калган код Rust'ту API'син өзгөртпөй
чакырышы үчүн bridge, порт дал келиши керек болгон так жүрүм-турум, аны чыгаруунун коопсуз жолу жана
порт качан даяр болгонун айткан gate.

Бирдей чыгыш жетиштүү эмес. Колдонмо сыртка эмне жөнөтөрүн да жазып алыңыз, ошондо катты эки жолу
жөнөткөн кайра жазылган нуска өтпөй калат:

<p align="center">
  <img src="assets/effects.gif" alt="--effects smtp менен app record ар бир катталуу үчүн бир саламдашуу катын жазып алат; бирдей чыгыш берген, бирок катты эки жолу жөнөткөн кайра жазылган нускада app replay 0/2 кейс өткөнүн жана effects боюнча ийгиликсиз болгонун билдирет, ал эми отчет ашыкча катты көрсөтөт." width="820">
  <br>
  <sub><code>app record --effects</code> жана <code>app replay</code> буйруктарынын чыныгы иштетилиши. <a href="whole-apps.md#side-effects-email-webhooks-and-databases">Кошумча таасирлер</a> бөлүмүн караңыз.</sub>
</p>

## Тилдер

| Тил | Бирдик | Bridge | Ким жазып алат жана кайра ойнотот |
| --- | --- | --- | --- |
| TypeScript/JavaScript | папка же файл | генерацияланган бинардык wire менен napi-rs же wasm-bindgen; класстар жана async да бар | Node, `.ts` баштапкы файлдарын түз иштетет |
| Python | пакет же модуль | PyO3 (abi3), түз айландыруу, типтелген exception'дор | Python, түп нусканы импорттойт |
| C | `.c` файлы | түп нуска аталыштары жана header менен `extern "C"` функциялары | shared library катары компиляцияланган түп нуска, ctypes аркылуу чакырылат, ар бир кейске бир fork |
| Go | пакет | типтелген cgo чакыруулары жана генерацияланган бинардык wire, түп нуска API'си бар пакетте | генерацияланган Go драйвери, өзүнчө модуль катары курулат |
| Ruby | модуль функцияларынан турган файл же папка | түп нуска модулдардын астындагы magnus кеңейтүүсү | генерацияланган Ruby драйвери |
| Java | `public static` методдорунан турган пакет | типтелген JNI жана генерацияланган бинардык wire, түп нуска методдору бар wrapper класстарында | генерацияланган reflection драйвери |
| PHP | функциялар жана `public static` методдорунан турган папка | кийинки кезекте: C ABI аркылуу PHP FFI | бүгүн бүт колдонмолор: `app record`, `app replay` жана front door, CLI'лар жана серверлер үчүн (`php -S`) |
| C# | `public static` методдорунан турган папка | кийинки кезекте: C ABI аркылуу P/Invoke | бүгүн бүт колдонмолор: `app record`, `app replay` жана front door, CLI'лар жана ASP.NET серверлери үчүн (`dotnet <app>.dll`) |

C'дан башка ар бир тил batch класстарын да алат (`--batch`): элементтер чектен бир жолу өтүп, Rust
ичинде калат, ал эми ар бир чакыруу алардын баарын бардык ядролордо иштетет.

Тил репозиторийден аныкталат (`go.mod`, `pom.xml`, `.csproj` же `.sln`, `pyproject.toml`, `Gemfile`,
`composer.json`, `package.json`, болбосо эң көп кездешкен баштапкы файлдар боюнча); `--lang` аны кол
менен көрсөтөт. Типтер баштапкы коддон алынат: TS типтери, Python hint'тери, C, Go, Java жана C#
сигнатуралары, Ruby YARD тегдери, PHP типтери жана phpdoc; сан катары колдонулган типсиз параметрлер
сан деп эсептелет.

PHP үчүн `analyze` PHP колдонмосун пландайт (Composer пакеттери жана PSR-4 autoloading,
loose comparison жана integer'дин float'ка толуп кетиши сыяктуу тобокелдиктер, crate'терге шайкештелген
Composer пакеттери), ал эми PHP CLI же серверин жазып алууга, кайра ойнотууга жана Rust front door'дун
артына коюуга болот. Анын бирдиктерин PHP FFI аркылуу Rust'ка порттоо кийинки кадам
([issue #2](https://github.com/erkamyaman/rust-anything/issues/2)).

C# да ушундай эле иштейт: `analyze` `.csproj` долбоорлорун жана алардын NuGet пакеттерин
окуйт (crate'терге шайкештелет), `using` директиваларын папкаларга чечет жана C# башкача кылган
нерселерди белгилейт (`decimal`, `dynamic`, LINQ'тун кийинкиге калтырылган суроо-талаптары, `Task`,
`lock`, reflection). C# CLI же ASP.NET серверин жазып алууга, кайра ойнотууга жана Rust front door'дун
артына коюуга болот; бирдиктерди P/Invoke аркылуу порттоо кийинки кезекте.

## Көбүрөөк билүү

- [Кантип иштейт](how-it-works.md): цикл, `bridge` жана `analyze` эмнени чечет, hot path'тар, ал эмнени кармай албайт, чыныгы порт эмнени талап кылат жана түп нускадагы өзгөрүүлөрдөн артта калбоо (`map`)
- [Бүт колдонмону порттоо](whole-apps.md): серверлер, CLI'лар жана китепканалар, front door'дон `app lift`'ке чейин
- [Мисалдар жана практикалык изилдөөлөр](examples.md): x18n жана алты тилдеги чыныгы код
- [Чыныгы порттор](real-world.md): json-server, Fuse.js, vercel/ms, бенчмарктары менен
- [Орнотуу, жаңыртуу жана иштеп чыгуу](install.md): орнотуунун бардык жолдору, Windows жана баштапкы коддон куруу
- [Көп берилүүчү суроолор](faq.md) · [Абалы](STATUS.md) · [MCP куралдары](tools.md)

## Демөөрчүлөр

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> акысыз жана ачык булак коддуу долбоор. Эгер ал ишиңизге жардам берсе, ага демөөрчү болууну карап көрүңүз.<br>
  Сиздин колдооңуз иштеп чыгууну улантууга жардам берет.
</p>

<p align="center">
  <a href="https://coderabbit.ai">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="Компания демөөрчүсү: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="Демөөрчүлөр: Santosh Yadav, Hakan Soytürk, Ali Yilmaz, Ashley Hunter, Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/%D0%94%D0%B5%D0%BC%D3%A9%D3%A9%D1%80%D1%87%D2%AF_%D0%B1%D0%BE%D0%BB%D1%83%D1%83-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="Демөөрчү болуу"></a>
</p>

## Салым кошуу

Issue'лар жана pull request'тер кубаттуу кабыл алынат; [CONTRIBUTING.md](../CONTRIBUTING.md) чөйрөнү
жөндөөнү, тесттер кайда жайгашарын жана жаңы тилди кантип кошууну түшүндүрөт. Дизайн чечимдери жана
алардын себептери [DECISIONS.md](../DECISIONS.md) ичинде. Чыныгы порттордун практикалык мисалдары
өзгөчө кубаттуу кабыл алынат. Коопсуздук көйгөйлөрү жөнүндө жеке кабарлаңыз
([SECURITY.md](../SECURITY.md)), ошондой эле [жүрүм-турум кодексин](../CODE_OF_CONDUCT.md) караңыз.

## Лицензия

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). Репозиторийиңиздеги порттор жана генерацияланган код сиздики.
