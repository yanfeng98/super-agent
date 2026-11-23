<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="Die rust-anything-Krabbe">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · Deutsch · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a></sub></p>
<p align="center"><sub>Diese Übersetzung wird von der Community gepflegt und kann hinter dem <a href="../README.md">englischen README</a> zurückliegen.</sub></p>

<p align="center"><b>Schreibe deine App in Rust neu und beweise, dass sie sich weiterhin genauso verhält.</b><br>
Es übersetzt keinen Code: Ihr Coding-Agent (oder Sie) schreibt das Rust, und rust-anything prüft es gegen das Original.</p>

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

<p align="center"><b><a href="#schnellstart">Schnellstart</a></b> · <a href="https://github.com/erkamyaman/rust-anything/releases">Releases</a> · <a href="#schnellstart">Claude Code plugin</a> · <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=rust-anything">MCP Registry</a> · <a href="real-world.md">Echte Portierungen</a> · <a href="faq.md#faq">FAQ</a><br>
<sub><a href="#sprachen">Sprachen</a> · <a href="how-it-works.md#how-it-works">So funktioniert es</a> · <a href="whole-apps.md#port-a-whole-app">Ganze Apps</a> · <a href="../LICENSE">MIT</a></sub><br>
Gemacht in der Türkei 🇹🇷 von <a href="https://github.com/erkamyaman">erKam Yaman</a> · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ Sponsere mich</a> · Gesponsert von <a href="https://coderabbit.link/erkamyaman"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center">Neu in 2.0: <a href="real-world.md#prettier">Prettier, nach Rust portiert</a>. 19x schneller · 15,861 von 17,224 aufgezeichneten Läufen komplett in Rust · alle Byte für Byte gleich</p>

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier und seine Rust-Portierung formatieren dieselben 59 echten Dateien, ein Prozess pro Datei: Prettier braucht 3.63 s, die Rust-Portierung 0.20 s, mit Byte für Byte derselben Ausgabe." width="820">
  <br>
  <sub>Prettier gegen seine Rust-Portierung an 59 echten Dateien aus ms, Fuse.js und json-server (Apple M4). Dieselbe Ausgabe, Byte für Byte. <a href="real-world.md#prettier">So wurde gemessen</a>.</sub>
</p>

<p align="center"><sub>PHP und C#: heute Aufzeichnen, Abspielen und die Front Door; Unit-Ports folgen.</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>Aufzeichnen</b><br>
      <code>record</code> führt das Original mit generierten Randfällen, semantischen Fallen und deinen eigenen Eingaben aus und speichert, was es zurückgibt oder wirft.
    </td>
    <td width="25%" valign="top">
      <b>Portieren</b><br>
      <code>bridge</code> erzeugt das Rust-Crate und einen Wrapper, sodass Aufrufer die ursprüngliche API behalten. Du oder dein Agent schreibt das Rust.
    </td>
    <td width="25%" valign="top">
      <b>Prüfen</b><br>
      <code>diff</code> spielt jeden aufgezeichneten Fall gegen das Rust ab, und nichts wird ausgeliefert, bevor alle übereinstimmen und <code>gate</code> besteht.
    </td>
    <td width="25%" valign="top">
      <b>Lift</b><br>
      Wenn die Wiedergabe ohne Fallback besteht und jede Einheit gate besteht, macht <code>app lift</code> daraus eine reine Rust-App. Das Original wird nie angefasst.
    </td>
  </tr>
</table>

**Schreibe deine gesamte App Stück für Stück in Rust neu.** rust-anything zeichnet auf, was deine
Node.js- (TypeScript/JavaScript), Python-, C-, Go-, Ruby- oder Java-App heute tut, und prüft dann
jeden Teil der Rust-Neuimplementierung gegen diese Aufzeichnung, bevor es weitergeht, bis die App
reines Rust ist. CLIs, HTTP-Server und Bibliotheken; dein Coding-Agent (oder du selbst) schreibt
das Rust. Es kann nur erfassen, was die Aufzeichnung abdeckt: siehe
[Was es nicht abfangen kann](how-it-works.md#what-it-cant-catch). Ganze Tools und Server, oder nur
der langsame Teil, ein Hot Path nach dem anderen:

| Echtes Projekt, gleiche Ausgabe wie das Original | Original | Mit rust-anything |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2, 2,360 JS/TS-Dateien über die Kommandozeile | 128.6 s | **6.9 s: 19x schneller** |
| Prettier, 6,305 Markdown-Dateien über die Kommandozeile | 352 s | **22.5 s: 15.6x schneller** |
| [json-server](real-world.md#json-server), ein ganzer Node-Server: Speicher im Leerlauf | 74 MB | **3.5 MB**, eine einzige 5.3 MB große Binärdatei |
| json-server, vom Start bis zur ersten Antwort | 79 bis 85 ms | **8 bis 10 ms** |
| [Fuse.js](real-world.md#fusejs)-Suche über 50,000 Strings, pro Anfrage | 91.2 ms | **2.8 ms: 32x schneller** |

Prettier: 15,861 von 17,224 aufgezeichneten Läufen formatieren vollständig in Rust, über 10 Sprachen
hinweg (JavaScript, TypeScript, JSX, JSON, GraphQL, YAML, Markdown, CSS, SCSS, Less), und alle 17,224
stimmen Byte für Byte mit Prettier überein, da alles noch nicht Portierte auf Prettier zurückfällt.
json-server: 514/514 aufgezeichnete HTTP-Fälle stimmen auf einer reinen Rust-Binärdatei überein.
Gemessen auf einem Apple M4, jeweils zwei Läufe, jede Ausgabe vor der Zeitmessung verglichen
([Details](real-world.md)). Rust für jede kleine Funktion einzeln aufzurufen ist meist *langsamer*
als ein JIT; rust-anything findet die Schleife, deren Auslagerung sich lohnt, und verschiebt sie.

## Nur der langsame Teil

Du musst nicht die ganze App portieren. Richte `go` auf einen Befehl, der zu langsam ist, und es
portiert die eine Funktion, die die Zeit kostet:

```sh
rust-anything go --agent claude -- python3 app.py
```

Es profiliert den Lauf, wählt die äußerste Funktion, deren Auslagerung sich lohnt (die Schleife, nicht
den kleinen Aufruf darin, als Batch, wo das Profil es nahelegt), zeichnet sie mit generierten Eingaben
und den Aufrufen deines Laufs auf und übergibt `src/port.rs` an deinen Agenten (`claude`, `codex` oder
einen beliebigen Befehl, der einen Prompt von stdin liest; ohne `--agent` schreibt es `TASK.md` für
dich und hält an). Es schaltet die Funktion erst ein, wenn jeder aufgezeichnete Fall übereinstimmt und
`gate` besteht. Danach:

```sh
rust-anything report rust/dedupe-rs     # REPORT.md und card.svg für den Pull Request
rust-anything disable rust/dedupe-rs    # zurück zum Original, Byte für Byte
```

Die [Fuse.js](real-world.md#fusejs)-Zeile oben hat genau diese Form: Fuses Score für einen String, als
Batch über die Liste, wurde nach Rust verschoben, während Objektschlüssel, erweiterte Suche und der Rest
in JavaScript bleiben, von 91.2 ms auf 2.8 ms pro Anfrage über 50,000 Strings. Probiere `go` an
[`tests/fixtures/go-fuzzy`](../tests/fixtures/go-fuzzy) aus, einem 20-zeiligen Python-Dedupe. Alle Wege,
eine Portierung ein- und auszuschalten: [Switch a port on and off](how-it-works.md#switch-a-port-on-and-off).

<p align="center">
  <img src="assets/json-server.gif" alt="json-server als Node-Server und als eine einzige Rust-Binärdatei: Speicher im Leerlauf 74 MB und 3.5 MB, vom Start bis zur ersten Antwort 79 bis 85 ms und 8 bis 10 ms, und 514 von 514 aufgezeichneten HTTP-Fällen stimmen überein." width="820">
  <br>
  <sub>json-server, ein ganzer Node-Server, gehoben zu einer reinen Rust-Binärdatei (Apple M4). <a href="real-world.md#json-server">So wurde gemessen</a>.</sub>
</p>

> rust-anything ist ein unabhängiges Projekt und steht in keiner Verbindung zum Rust Project oder
> der Rust Foundation und wird von diesen nicht unterstützt. „Rust“ ist eine Marke der Rust Foundation.

> [!WARNING]
> `record` und `app record` führen deinen Originalcode aus und rufen jede Funktion (oder die ganze App)
> mit vielen generierten Eingaben auf, und `app record` führt jeden Fall zweimal aus. `diff`, `gate` und
> `app replay` führen dieselben Fälle erneut aus, auf der Portierung oder dem Original. Code, der Dateien
> schreibt, mit einer Datenbank spricht oder das Netzwerk aufruft, tut das einmal pro Fall. Führe sie auf
> einer Kopie des Projekts aus, mit einer Testdatenbank und ohne Produktionszugangsdaten in der Umgebung.

## Schnellstart

Füge dies in Claude Code, Codex, Cursor, Gemini CLI oder OpenCode ein und lass deinen Agenten es installieren:

```text
Installiere rust-anything: Führe `cargo install rust-anything` aus und danach `rust-anything setup`.
Sag mir Bescheid, wann ich dich neu starten soll.
```

Oder selbst:

```sh
cargo install rust-anything   # von crates.io
rust-anything setup           # registriert es bei Claude Code, Codex, Cursor, Gemini CLI und OpenCode
```

Noch keine Rust-Toolchain? Vorgefertigte Binärdateien für macOS, Linux und Windows hängen an jedem
[GitHub-Release](https://github.com/erkamyaman/rust-anything/releases), und für JavaScript-Projekte
installiert `npm install -g rust-anything` dieselbe Binärdatei.

In Claude Code kannst du es auch als Plugin installieren (der MCP-Server und der Skill; das obige CLI
muss in deinem `PATH` liegen):

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

`setup` trägt den MCP-Server bei jedem gefundenen Agenten ein und installiert den Workflow-Skill dort,
wo der Agent Skills einliest. Deine übrigen Einstellungen bleiben erhalten, ein erneutes Ausführen ist
gefahrlos, und `--remove` macht es rückgängig. Frage deinen Agenten anschließend zum Beispiel:

- *"Schreibe dieses CLI mit rust-anything vollständig in Rust neu und beweise, dass es sich identisch verhält."*
- *"Diese Suche fühlt sich langsam an. Erstelle ein Profil mit rust-anything und verlagere die Hot Loop nach Rust."*
- *"Plane mit rust-anything eine Migration von `src/parser/` nach Rust; portiere die erste Einheit und verifiziere sie."*
- *"Vergleiche die Rust-Portierung in `rust/text-rs` im Benchmark mit dem Original und sag mir, ob wir sie ausliefern können."*

### Funktioniert mit

| Agent | `rust-anything setup` ergänzt |
| --- | --- |
| Claude Code | MCP-Server in `~/.claude.json`, Skill in `~/.claude/skills/` |
| Codex | MCP-Server in `~/.codex/config.toml`, Skill in `~/.codex/skills/` |
| Cursor | MCP-Server in `~/.cursor/mcp.json` |
| Gemini CLI | MCP-Server in `~/.gemini/settings.json` |
| OpenCode | MCP-Server in `~/.config/opencode/opencode.json` (`mcp.servers`), Skill in `~/.config/opencode/skills/` |
| Jeder andere MCP-Client | `rust-anything mcp` manuell als stdio-Server hinzufügen (siehe [Installation](install.md#install)) |

Bevor eine bestehende Datei geändert wird, legt `setup` daneben eine Kopie an (`<file>.rust-anything.bak`).

Um OpenCode manuell einzurichten, trage den Server in `~/.config/opencode/opencode.json` ein (`command`
ist der gesamte argv) und kopiere [`skills/rust-anything/`](../skills/rust-anything) nach
`~/.config/opencode/skills/rust-anything/`:

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

Oder führe die Schritte selbst aus:

```sh
rust-anything profile . -- node bench.mjs              # was heiß läuft und wer es aufruft
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# src/port.rs schreiben (oder vom Agenten schreiben lassen), dann:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # jeder aufgezeichnete Fall muss übereinstimmen
rust-anything bench rust/fuzzy-rs                      # schneller? Das Urteil sagt es dir
rust-anything gate rust/fuzzy-rs                       # bestanden heißt: bereit
```

rust-anything übersetzt keinen Code: Dein Coding-Agent (Claude Code, Codex, Cursor, OpenCode, …) oder
du selbst schreibst das Rust. rust-anything liefert genau das, was ein Agent nicht verlässlich erraten
kann: wo die Zeit bleibt, was zuerst und in welcher Reihenfolge portiert werden sollte, eine Bridge,
damit der übrige Code das Rust mit unveränderter API aufruft, das exakte Verhalten, dem die Portierung
entsprechen muss, einen sicheren Weg zur Auslieferung und ein Gate, das anzeigt, wann eine Portierung
fertig ist.

Dieselbe Ausgabe reicht nicht. Zeichne auch auf, was die App nach außen sendet, und eine Neuimplementierung, die zweimal eine E-Mail verschickt, fällt durch:

<p align="center">
  <img src="assets/effects.gif" alt="app record mit --effects smtp zeichnet pro Registrierung eine Willkommens-E-Mail auf; app replay einer Neuimplementierung, die dieselbe Ausgabe druckt, aber zweimal mailt, meldet 0/2 bestandene Fälle, fehlgeschlagen bei den Effekten, und der Bericht zeigt die zusätzliche E-Mail." width="820">
  <br>
  <sub>Ein echter Lauf von <code>app record --effects</code> und <code>app replay</code>. Siehe <a href="whole-apps.md#side-effects-email-webhooks-and-databases">Seiteneffekte</a>.</sub>
</p>

## Sprachen

| Sprache | Einheit | Bridge | Aufgezeichnet und wiedergegeben durch |
| --- | --- | --- | --- |
| TypeScript/JavaScript | Ordner oder Datei | napi-rs oder wasm-bindgen mit einem generierten binären Wire-Format; Klassen und async inklusive | Node, führt die `.ts`-Quellen direkt aus |
| Python | Paket oder Modul | PyO3 (abi3), direkte Konvertierung, typisierte Exceptions | Python, importiert das Original |
| C | `.c`-Datei | `extern "C"`-Funktionen mit den Originalnamen und einem Header | das Original, kompiliert als Shared Library, aufgerufen über ctypes, ein Fork pro Fall |
| Go | Paket | typisierte cgo-Aufrufe und ein generiertes binäres Wire-Format in einem Paket mit der Original-API | einen generierten Go-Treiber, gebaut als eigenes Modul |
| Ruby | Datei oder Ordner mit Modulfunktionen | eine magnus-Erweiterung unter den ursprünglichen Modulen | einen generierten Ruby-Treiber |
| Java | Paket mit `public static`-Methoden | typisiertes JNI und ein generiertes binäres Wire-Format in Wrapper-Klassen mit den Originalmethoden | einen generierten Reflection-Treiber |
| PHP | Ordner mit Funktionen und `public static`-Methoden | als Nächstes geplant: PHP FFI über ein C-ABI | heute schon ganze Apps: `app record`, `app replay` und die Front Door, für CLIs und Server (`php -S`) |
| C# | Ordner mit `public static`-Methoden | als Nächstes geplant: P/Invoke über ein C-ABI | heute schon ganze Apps: `app record`, `app replay` und die Front Door, für CLIs und ASP.NET-Server (`dotnet <app>.dll`) |

Jede Sprache außer C erhält außerdem Batch-Klassen (`--batch`): Die Elemente überqueren die Grenze
einmalig und bleiben in Rust, und jeder Aufruf verarbeitet sie alle auf allen Kernen.

Die Sprache wird anhand des Repositorys erkannt (`go.mod`, `pom.xml`, eine `.csproj` oder `.sln`,
`pyproject.toml`, `Gemfile`, `composer.json`, `package.json`, sonst die häufigsten Quelldateien);
`--lang` überschreibt dies. Typen stammen aus dem Quellcode: TS-Typen, Python-Type-Hints, C-, Go-,
Java- und C#-Signaturen, Ruby-YARD-Tags, PHP-Typen und phpdoc; untypisierte Parameter, die als Zahlen
verwendet werden, werden als Zahlen abgeleitet.

Für PHP plant `analyze` eine PHP-App (Composer-Pakete und PSR-4-Autoloading, Risiken
wie lose Vergleiche und Integer-Überläufe in Floats, Zuordnung von Composer-Paketen zu Crates), und ein
PHP-CLI oder -Server lässt sich aufzeichnen, wiedergeben und hinter eine Rust-Front-Door stellen. Das
Portieren einzelner Einheiten nach Rust über das PHP-FFI ist der nächste Schritt
([Issue #2](https://github.com/erkamyaman/rust-anything/issues/2)).

C# funktioniert auf dieselbe Weise: `analyze` liest `.csproj`-Projekte und ihre NuGet-Pakete
(auf Crates abgebildet), löst `using`-Direktiven zu Ordnern auf und markiert C#-Eigenheiten
(`decimal`, `dynamic`, verzögerte LINQ-Abfragen, `Task`, `lock`, Reflection). Ein C#-CLI oder
ASP.NET-Server lässt sich aufzeichnen, wiedergeben und hinter eine Rust-Front-Door stellen; das
Portieren einzelner Einheiten über P/Invoke folgt als Nächstes.

## Mehr erfahren

- [So funktioniert es](how-it-works.md): der Ablauf, was `bridge` und `analyze` entscheiden, Hot Paths, was es nicht abfangen kann, was eine echte Portierung erfordert und wie man mit dem Original Schritt hält (`map`)
- [Eine ganze App portieren](whole-apps.md): Server, CLIs und Bibliotheken, von der Front Door bis `app lift`
- [Beispiele und Fallstudien](examples.md): x18n und echter Code in sechs Sprachen
- [Echte Portierungen](real-world.md): json-server, Fuse.js, vercel/ms, mit Benchmarks
- [Installieren, aktualisieren und entwickeln](install.md): jede Installationsmethode, Windows und das Bauen aus dem Quellcode
- [FAQ](faq.md) · [Status](STATUS.md) · [MCP-Tools](tools.md)

## Sponsoren

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> ist kostenlos und Open Source. Wenn es dir bei der Arbeit hilft, ziehe bitte ein Sponsoring in Betracht.<br>
  Deine Unterstützung hält die Entwicklung am Laufen.
</p>

<p align="center">
  <a href="https://coderabbit.link/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="Unternehmenssponsor: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="Sponsoren: Santosh Yadav, Hakan Soytürk, Ali Yilmaz, Ashley Hunter, Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/Sponsor_werden-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="Sponsor werden"></a>
  <a href="https://buymeacoffee.com/erkamyaman"><img src="https://img.shields.io/badge/Buy_me_a_coffee-%E2%98%95-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee"></a>
</p>

## Mitwirken

Issues und Pull Requests sind willkommen; [CONTRIBUTING.md](../CONTRIBUTING.md) behandelt die
Einrichtung, wohin Tests gehören und wie man eine Sprache hinzufügt. Design-Entscheidungen und ihre
Gründe stehen in [DECISIONS.md](../DECISIONS.md). Fallstudien echter Portierungen sind besonders
willkommen. Sicherheitsprobleme bitte vertraulich melden ([SECURITY.md](../SECURITY.md)), und beachte
den [Verhaltenskodex](../CODE_OF_CONDUCT.md).

## Lizenz

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). Portierungen und generierter Code in
deinem Repository gehören dir.
