<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="Краб rust-anything">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><b>Перепишите своё приложение на Rust и докажите, что оно ведёт себя так же.</b></p>

<p align="center">Новое в 2.0: <a href="real-world.md#prettier">Prettier, портированный на Rust</a>. В 19x быстрее · 15,837 из 17,224 записанных запусков полностью на Rust · все совпадают байт в байт</p>

<p align="center">
  <a href="https://crates.io/crates/rust-anything"><img src="https://img.shields.io/crates/v/rust-anything?color=E2862A&label=crates.io" alt="Версия на crates.io"></a>
  <a href="https://www.npmjs.com/package/rust-anything"><img src="https://img.shields.io/npm/v/rust-anything?color=E2862A&label=npm" alt="Версия в npm"></a>
  <a href="https://github.com/erkamyaman/rust-anything/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/erkamyaman/rust-anything/ci.yml?branch=main&label=tests" alt="Тесты"></a>
  <a href="https://github.com/erkamyaman/rust-anything"><img src="https://img.shields.io/github/stars/erkamyaman/rust-anything?style=social" alt="Звёзды на GitHub"></a>
  <a href="https://github.com/AIshihzong/awesome-reverse-engineer-anything"><img src="https://img.shields.io/badge/Featured%20in-Awesome%20REA-f0b429" alt="Featured in Awesome REA"></a>
</p>

<p align="center">
  <a href="#быстрый-старт">Быстрый старт</a> ·
  <a href="#поддерживаемые-языки">Языки</a> ·
  <a href="real-world.md">Реальные порты</a> ·
  <a href="how-it-works.md#how-it-works">Как это работает</a> ·
  <a href="whole-apps.md#port-a-whole-app">Целые приложения</a> ·
  <a href="faq.md#faq">FAQ</a>
</p>

<p align="center">Сделано в Турции 🇹🇷, автор: <a href="https://github.com/erkamyaman">erKam Yaman</a> · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ Поддержать меня</a></p>

<p align="center">Спонсор: <a href="https://coderabbit.ai"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · Русский · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a></sub></p>
<p align="center"><sub>Этот перевод поддерживается сообществом и может отставать от <a href="../README.md">английского README</a>.</sub></p>
<table>
  <tr>
    <td width="25%" valign="top">
      <b>Запись</b><br>
      <code>record</code> запускает оригинал на сгенерированных граничных случаях, семантических ловушках и ваших собственных входных данных и сохраняет то, что он возвращает или выбрасывает.
    </td>
    <td width="25%" valign="top">
      <b>Порт</b><br>
      <code>bridge</code> генерирует Rust-крейт и обёртку, так что вызывающий код сохраняет исходный API. Rust пишете вы или ваш агент.
    </td>
    <td width="25%" valign="top">
      <b>Проверка</b><br>
      <code>diff</code> воспроизводит каждый записанный кейс на Rust, и ничего не выходит в релиз, пока все они не совпадут и не пройдёт <code>gate</code>.
    </td>
    <td width="25%" valign="top">
      <b>Подъём</b><br>
      Когда воспроизведение проходит без фолбэка и каждый модуль проходит gate, <code>app lift</code> превращает его в приложение на чистом Rust. Оригинал никогда не трогается.
    </td>
  </tr>
</table>

**Перепишите всё приложение на Rust, по одной части за раз.** rust-anything записывает поведение вашего текущего приложения на Node.js (TypeScript/JavaScript), Python, C, Go, Ruby или Java, а затем сверяет каждую переписанную на Rust часть с этой записью, прежде чем двигаться дальше, пока всё приложение не станет чистым Rust. Поддерживаются CLI, HTTP-серверы и библиотеки; код на Rust пишет ваш агент разработки (или вы сами). Инструмент способен отловить только то, что покрыто записью: см. [Что он не может отловить](how-it-works.md#what-it-cant-catch). Целые инструменты и серверы или только медленная часть, по одному горячему пути за раз:

| Реальный проект, тот же вывод, что у оригинала | Оригинал | С rust-anything |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2, 2,360 файлов JS/TS из командной строки | 128.6 s | **6.9 s: в 19x быстрее** |
| Prettier, 6,305 файлов Markdown из командной строки | 352 s | **22.5 s: в 15.6x быстрее** |
| [json-server](real-world.md#json-server), целый Node-сервер: память в простое | 74 MB | **3.5 MB**, один бинарник на 5.3 MB |
| json-server, от запуска до первого ответа | от 79 до 85 ms | **от 8 до 10 ms** |
| Поиск [Fuse.js](real-world.md#fusejs) по 50,000 строк, на запрос | 91.2 ms | **2.8 ms: в 32x быстрее** |

Prettier: 15,837 из 17,224 записанных запусков полностью форматируются на Rust на 10 языках (JavaScript, TypeScript, JSX, JSON, GraphQL, YAML, Markdown, CSS, SCSS, Less), и все 17,224 совпадают с Prettier байт в байт, поскольку всё, что ещё не портировано, откатывается на Prettier. json-server: 514/514 записанных HTTP-кейсов совпадают на бинарнике на чистом Rust. Замеры выполнены на Apple M4, по два прогона, каждый вывод сравнивается перед замером времени ([подробности](real-world.md)). Вызывать Rust по одному разу на каждую мелкую функцию обычно *медленнее*, чем JIT; rust-anything находит цикл, который действительно стоит перенести, и переносит его.

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier и его порт на Rust форматируют одни и те же 59 реальных файлов, по одному процессу на файл: Prettier тратит 3.63 s, порт на Rust 0.20 s, а вывод совпадает байт в байт." width="820">
  <br>
  <sub>Prettier против своего порта на Rust на 59 реальных файлах из ms, Fuse.js и json-server (Apple M4). Одинаковый вывод, байт в байт. <a href="real-world.md#prettier">Как измеряли</a>.</sub>
</p>

<p align="center">
  <img src="assets/json-server.gif" alt="json-server как Node-сервер и как один бинарник на Rust: память в простое 74 MB и 3.5 MB, от запуска до первого ответа от 79 до 85 ms и от 8 до 10 ms, и 514 из 514 записанных HTTP-кейсов совпадают." width="820">
  <br>
  <sub>json-server, целый Node-сервер, перенесённый в бинарник на чистом Rust (Apple M4). <a href="real-world.md#json-server">Как измеряли</a>.</sub>
</p>

> rust-anything является независимым проектом, не связанным с Rust Project или Rust Foundation и не одобренным ими. «Rust» является товарным знаком Rust Foundation.

> [!WARNING]
> `record` и `app record` запускают ваш исходный код, вызывая каждую функцию (или всё приложение) на
> множестве сгенерированных входных данных, а `app record` прогоняет каждый кейс дважды. `diff`, `gate` и `app replay`
> снова запускают те же кейсы, на порте или на оригинале. Код, который пишет файлы, обращается к
> базе данных или ходит в сеть, делает это по разу на каждый кейс. Запускайте их на копии проекта, с
> тестовой базой данных и без боевых учётных данных в окружении.

## Быстрый старт

Вставьте это в Claude Code, Codex, Cursor, Gemini CLI или OpenCode, и пусть ваш агент всё установит:

```text
Установи rust-anything: выполни `cargo install rust-anything`, затем `rust-anything setup`.
Скажи мне, когда тебя нужно будет перезапустить.
```

Или установите сами:

```sh
cargo install rust-anything   # из crates.io
rust-anything setup           # регистрирует инструмент в Claude Code, Codex, Cursor, Gemini CLI и OpenCode
```

Ещё не установлен тулчейн Rust? Готовые бинарные файлы для macOS, Linux и Windows прикреплены к каждому [релизу на GitHub](https://github.com/erkamyaman/rust-anything/releases), а для JavaScript-проектов тот же бинарник устанавливается командой `npm install -g rust-anything`.

В Claude Code инструмент также можно установить как плагин (MCP-сервер и навык; указанный выше CLI должен быть в `PATH`):

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

`setup` добавляет MCP-сервер во все найденные агенты и устанавливает навык рабочего процесса туда, откуда агент читает навыки. Остальные настройки сохраняются, повторный запуск безопасен, а `--remove` отменяет изменения. После этого можно давать агенту задачи, например:

- *«Перепиши этот CLI на Rust с помощью rust-anything от начала до конца и докажи, что он ведёт себя так же.»*
- *«Этот поиск ощущается медленным. Профилируй его через rust-anything и перенеси горячий цикл в Rust.»*
- *«Спланируй миграцию `src/parser/` на Rust с rust-anything; перенеси первый модуль и докажи, что он работает.»*
- *«Сравни производительность Rust-порта в `rust/text-rs` с оригиналом и скажи, стоит ли его выкатывать.»*

### Поддерживаемые агенты

| Агент | Что добавляет `rust-anything setup` |
| --- | --- |
| Claude Code | MCP-сервер в `~/.claude.json`, навык в `~/.claude/skills/` |
| Codex | MCP-сервер в `~/.codex/config.toml`, навык в `~/.codex/skills/` |
| Cursor | MCP-сервер в `~/.cursor/mcp.json` |
| Gemini CLI | MCP-сервер в `~/.gemini/settings.json` |
| OpenCode | MCP-сервер в `~/.config/opencode/opencode.json` (`mcp.servers`), навык в `~/.config/opencode/skills/` |
| Любой другой MCP-клиент | добавьте `rust-anything mcp` как stdio-сервер вручную (см. [Установка](install.md#install)) |

Перед изменением существующего файла `setup` сохраняет копию рядом с ним (`<file>.rust-anything.bak`).

Чтобы подключить OpenCode вручную, добавьте сервер в `~/.config/opencode/opencode.json` (`command` содержит весь argv) и скопируйте [`skills/rust-anything/`](../skills/rust-anything) в `~/.config/opencode/skills/rust-anything/`:

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

Либо выполняйте шаги вручную:

```sh
rust-anything profile . -- node bench.mjs              # что тормозит и кто это вызывает
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# напишите src/port.rs (или поручите агенту), затем:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # каждый записанный кейс должен совпасть
rust-anything bench rust/fuzzy-rs                      # стало быстрее? вердикт покажет
rust-anything gate rust/fuzzy-rs                       # готово, когда эта проверка пройдена
```

rust-anything не переводит код: код на Rust пишете вы или ваш агент разработки (Claude Code, Codex, Cursor, OpenCode и др.). rust-anything даёт то, что агент не может надёжно угадать: куда уходит время, что переносить первым и в каком порядке, мост, чтобы остальной код вызывал Rust с неизменным API, точное поведение, которому порт обязан соответствовать, безопасный способ выкатки и гейт, который говорит, когда порт готов.

Одинакового вывода недостаточно. Записывайте и то, что приложение отправляет наружу, и переписанная версия, которая шлёт письмо дважды, не пройдёт проверку:

<p align="center">
  <img src="assets/effects.gif" alt="app record с --effects smtp записывает одно приветственное письмо на каждую регистрацию; app replay переписанной версии, которая выводит то же самое, но шлёт письмо дважды, сообщает, что прошло 0/2 кейсов из-за сбоя на эффектах, а отчёт показывает лишнее письмо." width="820">
  <br>
  <sub>Реальный запуск <code>app record --effects</code> и <code>app replay</code>. См. <a href="whole-apps.md#side-effects-email-webhooks-and-databases">Побочные эффекты</a>.</sub>
</p>

## Поддерживаемые языки

| Язык | Единица переноса | Мост | Чем записывается и воспроизводится |
| --- | --- | --- | --- |
| TypeScript/JavaScript | каталог или файл | napi-rs или wasm-bindgen со сгенерированным бинарным протоколом; включая классы и async | Node, напрямую выполняющий исходники `.ts` |
| Python | пакет или модуль | PyO3 (abi3), прямое преобразование, типизированные исключения | Python, импортирующий оригинал |
| C | файл `.c` | функции `extern "C"` с исходными именами и заголовочный файл | оригинал, скомпилированный в разделяемую библиотеку, вызов через ctypes, отдельный fork на кейс |
| Go | пакет | типизированные вызовы cgo и сгенерированный бинарный протокол в пакете с исходным API | сгенерированный Go-драйвер, собранный как отдельный модуль |
| Ruby | файл или каталог с функциями модуля | расширение magnus в исходных модулях | сгенерированный Ruby-драйвер |
| Java | пакет методов `public static` | типизированный JNI и сгенерированный бинарный протокол в классах-обёртках с исходными методами | сгенерированный драйвер на рефлексии |
| PHP | каталог функций и методов `public static` | далее: PHP FFI поверх C ABI | целые приложения уже сейчас: `app record`, `app replay` и входная точка для CLI и серверов (`php -S`) |
| C# | каталог методов `public static` | далее: P/Invoke поверх C ABI | целые приложения уже сейчас: `app record`, `app replay` и входная точка для CLI и серверов ASP.NET (`dotnet <app>.dll`) |

Для каждого языка, кроме C, также генерируются пакетные классы (`--batch`): элементы пересекают границу один раз и остаются в Rust, а каждый вызов обрабатывает их все на всех ядрах.

Язык определяется по репозиторию (`go.mod`, `pom.xml`, `.csproj` или `.sln`, `pyproject.toml`, `Gemfile`, `composer.json`, `package.json`, иначе по преобладающим исходным файлам); флаг `--lang` переопределяет автоопределение. Типы берутся из исходного кода: типы TS, аннотации типов Python, сигнатуры C, Go, Java и C#, теги YARD в Ruby, типы и phpdoc в PHP; нетипизированные параметры, используемые как числа, выводятся как числовые.

Для PHP `analyze` планирует PHP-приложение (пакеты Composer и автозагрузка PSR-4, риски вроде нестрогого сравнения и переполнения целых чисел во float, сопоставление пакетов Composer с крейтами), а PHP CLI или сервер можно записать, воспроизвести и поставить за входную точку на Rust. Перенос отдельных модулей в Rust через PHP FFI станет следующим этапом ([issue #2](https://github.com/erkamyaman/rust-anything/issues/2)).

C# работает аналогичным образом: `analyze` читает проекты `.csproj` и их пакеты NuGet (сопоставляемые с крейтами), разрешает директивы `using` в каталоги и отмечает особенности C# (`decimal`, `dynamic`, отложенные запросы LINQ, `Task`, `lock`, рефлексия). C# CLI или сервер ASP.NET можно записать, воспроизвести и поставить за входную точку на Rust; перенос отдельных модулей через P/Invoke появится далее.

## Подробнее

- [Как это работает](how-it-works.md): цикл работы, что решают `bridge` и `analyze`, горячие пути, что инструмент не может отловить, чего требует реальный порт и как не отставать от оригинала (`map`)
- [Портирование целого приложения](whole-apps.md): серверы, CLI и библиотеки, от входной точки до `app lift`
- [Примеры и разборы кейсов](examples.md): x18n и реальный код на шести языках
- [Реальные порты](real-world.md): json-server, Fuse.js, vercel/ms, с бенчмарками
- [Установка, обновление и разработка](install.md): все способы установки, Windows и сборка из исходников
- [FAQ](faq.md) · [Статус](STATUS.md) · [Инструменты MCP](tools.md)

## Спонсоры

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> бесплатен и распространяется с открытым исходным кодом. Если он помогает вам в работе, пожалуйста, подумайте о спонсорстве.<br>
  Ваша поддержка помогает продолжать разработку.
</p>

<p align="center">
  <a href="https://coderabbit.ai">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="Спонсор-компания: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="Спонсоры: Santosh Yadav, Hakan Soytürk, Ali Yilmaz, Ashley Hunter, Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/%D0%A1%D1%82%D0%B0%D1%82%D1%8C_%D1%81%D0%BF%D0%BE%D0%BD%D1%81%D0%BE%D1%80%D0%BE%D0%BC-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="Стать спонсором"></a>
</p>

## Участие в разработке

Будем рады вашим issue и pull request; в [CONTRIBUTING.md](../CONTRIBUTING.md) описаны настройка окружения, расположение тестов и добавление нового языка. Проектные решения и их причины собраны в [DECISIONS.md](../DECISIONS.md). Особенно приветствуются разборы реальных портов. Проблемы безопасности сообщайте приватно ([SECURITY.md](../SECURITY.md)), а также ознакомьтесь с [кодексом поведения](../CODE_OF_CONDUCT.md).

## Лицензия

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). Порты и сгенерированный код в вашем репозитории принадлежат вам.
