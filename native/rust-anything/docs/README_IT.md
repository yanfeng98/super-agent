<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="Il granchio di rust-anything">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a> · <a href="README_ES.md">Español</a> · Italiano</sub></p>
<p align="center"><sub>Questa traduzione è mantenuta dalla community e può restare indietro rispetto al <a href="../README.md">README in inglese</a>.</sub></p>

<p align="center"><b>Riscrivi la tua applicazione in Rust e dimostra che si comporta ancora allo stesso modo.</b><br>
Puntalo sulla tua applicazione: registra ciò che fa l'originale, il tuo agente di programmazione scrive il Rust un pezzo alla volta,<br>
e ogni pezzo viene verificato rispetto a quella registrazione prima di proseguire. Non traduce codice: dimostra la riscrittura.</p>

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

<p align="center"><b><a href="#avvio-rapido">Installa</a></b> · <a href="https://github.com/erkamyaman/rust-anything/releases">Release</a> · <a href="#avvio-rapido">Plugin per Claude Code</a> · <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=rust-anything">Registro MCP</a> · <a href="real-world.md">Port reali</a> · <a href="faq.md#faq">FAQ</a><br>
<sub><a href="#linguaggi">Linguaggi</a> · <a href="how-it-works.md#how-it-works">Come funziona</a> · <a href="whole-apps.md#port-a-whole-app">Applicazioni intere</a> · <a href="#sponsor">Sponsor</a> · <a href="../LICENSE">MIT</a></sub><br>
Fatto in Türkiye 🇹🇷 da <a href="https://github.com/erkamyaman">erKam Yaman</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ Sostienimi</a> · Con il supporto di <a href="https://coderabbit.link/erkamyaman"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center">Novità della 2.0: <a href="real-world.md#prettier">Prettier, portato in Rust</a>. 19 volte più veloce · 15.861 delle 17.224 esecuzioni registrate interamente in Rust · ogni esecuzione identica byte per byte</p>

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier e il suo port in Rust formattano gli stessi 59 file reali, un processo per file: Prettier impiega 3,63 s e il port in Rust 0,20 s, con lo stesso output byte per byte." width="820">
  <br>
  <sub>Prettier contro il suo port in Rust su 59 file reali di ms, Fuse.js e json-server (Apple M4). Stesso output, byte per byte. <a href="real-world.md#prettier">Come è stato misurato</a>.</sub>
</p>

<p align="center"><sub>PHP e C#: oggi registrazione, ripetizione e front door; i port delle unità arrivano dopo.</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>Registra</b><br>
      <code>record</code> esegue l'originale su casi limite generati, trappole semantiche e i tuoi input, e salva ciò che restituisce o lancia.
    </td>
    <td width="25%" valign="top">
      <b>Porta</b><br>
      <code>bridge</code> genera il crate Rust e un wrapper, così chi chiama mantiene l'API originale. Il Rust lo scrivi tu o il tuo agente.
    </td>
    <td width="25%" valign="top">
      <b>Verifica</b><br>
      <code>diff</code> ripete ogni caso registrato contro il Rust, e nulla viene pubblicato finché non coincidono tutti e <code>gate</code> non passa.
    </td>
    <td width="25%" valign="top">
      <b>Solleva</b><br>
      Quando la ripetizione passa senza ricorrere all'originale e ogni unità passa gate, <code>app lift</code> la rende un'applicazione Rust pura. L'originale non viene mai toccato.
    </td>
  </tr>
</table>

**Riscrivi tutta la tua applicazione in Rust, un pezzo alla volta.** rust-anything registra ciò che la
tua applicazione Node.js (TypeScript/JavaScript), Python, C, Go, Ruby, Java o Kotlin fa oggi, poi
verifica ogni pezzo della riscrittura Rust rispetto a quella registrazione prima di proseguire, fino
a che l'applicazione è Rust puro. CLI, server HTTP e librerie; il Rust lo scrive il tuo agente di
programmazione (o tu). Si appoggia a [napi-rs](https://github.com/napi-rs/napi-rs), [PyO3](https://github.com/PyO3/pyo3) e [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen), che portano le chiamate tra il tuo linguaggio e Rust; rust-anything decide cosa spostare e dimostra che corrisponde. Può cogliere solo ciò che la registrazione copre: vedi
[Cosa non può cogliere](how-it-works.md#what-it-cant-catch). Strumenti e server interi, oppure solo
la parte lenta, un percorso caldo alla volta:

| Progetto reale, stesso output dell'originale | Originale | Con rust-anything |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2, 2.360 file JS/TS dalla riga di comando | 128,6 s | **6,9 s: 19 volte più veloce** |
| Prettier, 6.305 file Markdown dalla riga di comando | 352 s | **22,5 s: 15,6 volte più veloce** |
| [json-server](real-world.md#json-server), un server Node intero: memoria a riposo | 74 MB | **3,5 MB**, un binario da 5,3 MB |
| json-server, dall'avvio alla prima risposta | da 79 a 85 ms | **da 8 a 10 ms** |
| [Fuse.js](real-world.md#fusejs) ricerca su 50.000 stringhe, per query | 91,2 ms | **2,8 ms: 32 volte più veloce** |

Prettier: 15.861 delle 17.224 esecuzioni registrate formattano interamente in Rust su 10 linguaggi
(JavaScript, TypeScript, JSX, JSON, GraphQL, YAML, Markdown, CSS, SCSS, Less), e tutte le 17.224
coincidono con Prettier byte per byte, perché ciò che non è ancora portato ricade su Prettier.
json-server: 514 casi HTTP registrati su 514 coincidono su un binario Rust puro. Misurato su un
Apple M4, due esecuzioni per ciascuno, confrontando ogni output prima di cronometrare
([dettagli](real-world.md)). Chiamare Rust una volta per una funzione piccola è di solito *più
lento* di un JIT; rust-anything trova il ciclo che vale la pena spostare, e lo sposta. Rispetto al C il guadagno è la
sicurezza, non la velocità: [libavutil di ffmpeg](real-world.md#ffmpeg) portato in Rust corrisponde in
tutte le 226.447 chiamate registrate e va alla velocità del C (18 benchmark su 42 uguali, 8 più
veloci e 16 più lenti dal 3 al 40 %).

## Solo la parte lenta

Non devi portare tutta l'applicazione. Punta `go` su un comando troppo lento, e porta l'unica funzione
che si prende il tempo:

```sh
rust-anything go --agent claude -- python3 app.py
```

Profila l'esecuzione, sceglie la funzione più esterna che vale la pena spostare (il ciclo, non la
piccola chiamata al suo interno, a lotti dove il profilo lo indica), la registra su input generati più
le chiamate che fa la tua esecuzione, e consegna `src/port.rs` al tuo agente (`claude`, `codex` o
qualsiasi comando che legga un prompt da stdin; senza `--agent` scrive `TASK.md` per te e si ferma).
Attiva la funzione solo quando ogni caso registrato coincide e `gate` passa. Poi:

```sh
rust-anything report rust/dedupe-rs     # REPORT.md e card.svg per la pull request
rust-anything disable rust/dedupe-rs    # di nuovo l'originale, byte per byte
```

La riga [Fuse.js](real-world.md#fusejs) qui sopra ha questa forma: il punteggio di Fuse per una
stringa, a lotti sulla lista, spostato in Rust mentre le chiavi degli oggetti, la ricerca estesa e il
resto restano in JavaScript, da 91,2 ms a 2,8 ms per query su 50.000 stringhe. Prova `go` su
[`tests/fixtures/go-fuzzy`](../tests/fixtures/go-fuzzy), un dedupe Python di 20 righe. Tutti i modi per
attivare e disattivare un port: [Switch a port on and off](how-it-works.md#switch-a-port-on-and-off).

<p align="center">
  <img src="assets/json-server.gif" alt="json-server come server Node e come un unico binario Rust: memoria a riposo 74 MB e 3,5 MB, dall'avvio alla prima risposta 79 a 85 ms e 8 a 10 ms, e 514 casi HTTP registrati su 514 coincidono." width="820">
  <br>
  <sub>json-server, un server Node intero, sollevato a un binario Rust puro (Apple M4). <a href="real-world.md#json-server">Come è stato misurato</a>.</sub>
</p>

> rust-anything è un progetto indipendente, non affiliato né approvato dal Rust Project o dalla Rust
> Foundation. «Rust» è un marchio della Rust Foundation.

> [!WARNING]
> `record` e `app record` eseguono il tuo codice originale, chiamando ogni funzione (o l'intera
> applicazione) su molti input generati, e `app record` esegue ogni caso due volte. `diff`, `gate` e
> `app replay` rieseguono gli stessi casi, sul port o sull'originale. Il codice che scrive file,
> parla con un database o chiama la rete lo fa una volta per caso. Eseguili su una copia del
> progetto, con un database di prova e nessuna credenziale di produzione nell'ambiente.

## Installare i port

Il port di ESLint è un comando npm. Esegue il port in Rust per ciò che copre ed ESLint 10.15.0,
fissato come dipendenza, per tutto il resto, così l'output è quello di ESLint in ogni caso:

```sh
npm install -g eslint-rs     # ESLint 10.15.0
```

| Comando | Risposto in Rust | Passato a ESLint 10.15.0 |
| --- | --- | --- |
| `eslint-rs` | `--no-config-lookup --rule ... --stdin` con le 90 regole portate (output `stylish` e `json`, correzioni, suggerimenti) | ricerche di `eslint.config.*` (`eslint .`), file e cartelle, le altre 202 regole del core, plugin, parser, gli altri formatter, errori di parsing |

Il binario esiste per macOS (arm64, x64) e Linux (x64, arm64, glibc). Su Windows e altrove, o con
`RUST_ANYTHING_MODE=old`, `eslint-rs` esegue ESLint: stesso output, nessun guadagno di velocità.
Installato dai suoi tarball su macOS arm64, `eslint-rs` corrisponde a ESLint 10.15.0 installato
accanto in tutte le 10.585 esecuzioni registrate ([come](packaging.md#a-whole-cli-front-door)).

Il port di Prettier è impacchettato allo stesso modo (`prettier-rs`, Prettier 3.6.2) e ripete 17.224
esecuzioni registrate su 17.224 dai suoi tarball, con il 7,9 % passato a Prettier, ma non è su npm.
`rust-anything package` produce questo tipo di pacchetto da qualsiasi front door di una CLI.

## Avvio rapido

Incolla questo in Claude Code, Codex, Cursor, Gemini CLI o OpenCode e lascia che il tuo agente lo installi:

```text
Install rust-anything: run `cargo install rust-anything`, then `rust-anything setup`.
Tell me when to restart you.
```

Oppure fallo tu:

```sh
cargo install rust-anything   # da crates.io
rust-anything setup           # lo registra in Claude Code, Codex, Cursor, Gemini CLI e OpenCode
```

Non hai ancora un toolchain Rust? I binari precompilati per macOS, Linux e Windows sono allegati a
ogni [release GitHub](https://github.com/erkamyaman/rust-anything/releases), e per i progetti
JavaScript `npm install -g rust-anything` installa lo stesso binario.

In Claude Code puoi anche installarlo come plugin (il server MCP e la skill; la CLI qui sopra deve
essere nel tuo `PATH`):

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

In Codex, dal marketplace degli agenti di questo repository
(`.agents/plugins/marketplace.json`, lo stesso plugin sotto `plugin/`, con la skill e il server MCP):

```sh
codex plugin marketplace add erkamyaman/rust-anything
codex plugin add rust-anything@rust-anything
```

`setup` aggiunge il server MCP a ogni agente che trova e installa la skill del flusso di lavoro dove
l'agente legge le skill. Conserva le tue altre impostazioni, si può rieseguire senza rischi e
`--remove` lo annulla. Poi chiedi al tuo agente, per esempio:

- *"Riscrivi questa CLI in Rust con rust-anything, da capo a fondo, e dimostra che si comporta allo stesso modo."*
- *"Questa ricerca è lenta. Profilala con rust-anything e sposta il ciclo caldo in Rust."*
- *"Pianifica una migrazione di `src/parser/` a Rust con rust-anything; porta la prima unità e dimostralo."*
- *"Confronta il port Rust in `rust/text-rs` con l'originale e dimmi se vale la pena pubblicarlo."*

### Funziona con

| Agente | Cosa aggiunge `rust-anything setup` |
| --- | --- |
| Claude Code | server MCP in `~/.claude.json`, skill in `~/.claude/skills/` |
| Codex | server MCP in `~/.codex/config.toml`, skill in `~/.codex/skills/` |
| Cursor | server MCP in `~/.cursor/mcp.json` |
| Gemini CLI | server MCP in `~/.gemini/settings.json` |
| OpenCode | server MCP in `~/.config/opencode/opencode.json` (`mcp.servers`), skill in `~/.config/opencode/skills/` |
| Qualsiasi altro client MCP | aggiungi `rust-anything mcp` come server stdio a mano (vedi [Installa](install.md#install)) |

Prima di modificare un file che esiste, `setup` ne salva una copia accanto (`<file>.rust-anything.bak`).

Per configurare OpenCode a mano, metti il server in `~/.config/opencode/opencode.json` (`command` è
l'intero argv) e copia [`skills/rust-anything/`](../skills/rust-anything) in
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

Oppure esegui i passi tu:

```sh
rust-anything profile . -- node bench.mjs              # cos'è caldo, e chi lo chiama
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# scrivi src/port.rs (o lascia fare al tuo agente), poi:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # ogni caso registrato deve coincidere
rust-anything fuzz rust/fuzzy-rs --time 60 --keep      # input nuovi: ciò che la registrazione non copre
rust-anything bench rust/fuzzy-rs                      # più veloce? lo dice il verdetto
rust-anything gate rust/fuzzy-rs                       # finito significa che questo passa
```

rust-anything non traduce codice: il Rust lo scrivono il tuo agente di programmazione (Claude Code,
Codex, Cursor, OpenCode, ...) o tu. rust-anything fornisce ciò che un agente non può indovinare in
modo affidabile: dove va il tempo, cosa portare prima e in quale ordine, un ponte perché il resto del
codice chiami il Rust con la sua API invariata, il comportamento esatto che il port deve eguagliare,
un modo sicuro per pubblicarlo e un cancello che dice quando un port è finito.

Lo stesso output non basta. Registra anche ciò che l'applicazione invia, e una riscrittura che manda
l'email due volte fallisce:

<p align="center">
  <img src="assets/effects.gif" alt="app record con --effects smtp registra un'email di benvenuto per ogni iscrizione; app replay di una riscrittura che stampa lo stesso output ma invia l'email due volte riporta 0/2 casi superati, fallisce sugli effetti e il report mostra l'email in più." width="820">
  <br>
  <sub>Un'esecuzione reale di <code>app record --effects</code> e <code>app replay</code>. Vedi <a href="whole-apps.md#side-effects-email-webhooks-and-databases">Effetti collaterali</a>.</sub>
</p>

## Linguaggi

| Linguaggio | Unità | Ponte | Registrato e ripetuto da |
| --- | --- | --- | --- |
| TypeScript/JavaScript | cartella o file | napi-rs o wasm-bindgen con un wire binario generato; classi e async inclusi | Node, eseguendo i sorgenti `.ts` |
| Python | pacchetto o modulo | PyO3 (abi3), conversione diretta, eccezioni tipizzate | Python, importando l'originale |
| C | file `.c` | funzioni `extern "C"` con i nomi originali e un header | l'originale compilato in una libreria condivisa, chiamato via ctypes, un fork per caso |
| Go | pacchetto | chiamate cgo tipizzate e un wire binario generato, in un pacchetto con l'API originale | un driver Go generato, compilato come modulo a sé |
| Ruby | file o cartella di funzioni di modulo | un'estensione magnus sotto i moduli originali | un driver Ruby generato |
| Java | pacchetto di metodi `public static` | JNI tipizzato e un wire binario generato, in classi wrapper con i metodi originali | un driver di reflection generato |
| Kotlin | modulo Gradle di funzioni di primo livello e metodi `@JvmStatic` | il ponte Java: JNI tipizzato in classi wrapper Java che chiamano gli static Kotlin compilati | un driver di reflection generato, con i sorgenti Kotlin compilati da `kotlinc` |
| PHP | cartella di funzioni e metodi `public static` | in arrivo: FFI di PHP su una ABI C | oggi applicazioni intere: `app record`, `app replay` e la front door, per CLI e server (`php -S`) |
| C# | cartella di metodi `public static` | in arrivo: P/Invoke su una ABI C | oggi applicazioni intere: `app record`, `app replay` e la front door, per CLI e server ASP.NET (`dotnet <app>.dll`) |

Ogni linguaggio tranne il C riceve anche classi batch (`--batch`): gli elementi attraversano una
volta sola e restano in Rust, e ogni chiamata li esegue tutti su tutti i core.

Il linguaggio viene rilevato dal repository (`go.mod`, `pom.xml`, `build.gradle`/`build.gradle.kts`,
un `.csproj` o `.sln`, `pyproject.toml`, `Gemfile`, `composer.json`, `package.json`, altrimenti i file
sorgente più comuni); un modulo Gradle con sorgenti `.kt` e nessun sorgente `.java` è Kotlin, i moduli
misti restano Java. `--lang` lo sovrascrive. I tipi vengono dal codice: tipi TS, hint di Python,
firme C, Go, Java, Kotlin e C#, tag YARD di Ruby, tipi e phpdoc di PHP; i parametri senza tipo usati
come numeri vengono inferiti come numeri.

Per Android, `rust-anything build <crate> --profile android` compila la stessa libreria JNI con l'NDK
per `arm64-v8a`, `armeabi-v7a` e `x86_64` (o i valori `--abi` che passi) e la installa in
`<crate>/jniLibs/<abi>/`; `--api` imposta il livello API (minimo 26, che è ciò che serve al runtime
Java generato). Trova l'NDK in `ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT` o nell'SDK, richiede i target
Rust (`rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`),
collega le librerie a 64 bit con allineamento di pagina a 16 KB e verifica che ogni punto di ingresso
JNI sia esportato. Gradle cerca in `src/main/jniLibs` per impostazione predefinita, quindi passa
`--jni-libs app/src/main/jniLibs` o aggiungi `<crate>/jniLibs` a `sourceSets.main.jniLibs.srcDirs`.
`rust-anything record <crate> --profile android` esegue l'unità Kotlin originale sotto ART su un
dispositivo collegato (adb), converte in dex il driver generato con `d8` delle build-tools dell'SDK e
riporta i risultati come lo stesso `cases.jsonl`; `diff <crate> --profile android` li ripete contro
il `.so` compilato per l'ABI del dispositivo. Entrambi richiedono `adb`, un dispositivo, `kotlinc`,
un SDK con build-tools e una piattaforma, e `--device <serial>` quando è collegato più di un
dispositivo. Il profilo copre le unità di logica: la UI di Activity/Compose/XML, il ciclo di vita e
le classi solo-SDK restano in Kotlin, e ogni chiamata attraversa JNI, quindi per i cicli caldi è
meglio usare i wrapper batch generati. Su un host, `record` e `diff` continuano a usare la JVM desktop.

Per PHP, `analyze` pianifica un'applicazione PHP (pacchetti Composer e autoload PSR-4, rischi come il
confronto lasco e l'overflow degli interi in virgola mobile, pacchetti Composer mappati su crate) e
una CLI o un server PHP può essere registrato, ripetuto e messo dietro una front door Rust. Portare le
sue unità in Rust tramite la FFI di PHP è il passo successivo
([issue #2](https://github.com/erkamyaman/rust-anything/issues/2)).

C# funziona allo stesso modo: `analyze` legge i progetti `.csproj` e i loro pacchetti NuGet (mappati
su crate), risolve le direttive `using` in cartelle e segnala ciò che C# fa diversamente (`decimal`,
`dynamic`, le query differite di LINQ, `Task`, `lock`, reflection). Una CLI C# o un server ASP.NET può
essere registrato, ripetuto e messo dietro una front door Rust; portare le unità tramite P/Invoke
arriva dopo.

## Saperne di più

- [Come funziona](how-it-works.md): il ciclo, cosa decidono `bridge` e `analyze`, i percorsi caldi, cosa non può cogliere, quanto costa un port reale e come seguire l'originale (`map`)
- [Portare un'applicazione intera](whole-apps.md): server, CLI e librerie, dalla front door a `app lift`
- [Esempi e casi di studio](examples.md): x18n e codice reale in sei linguaggi
- [Port reali](real-world.md): json-server, Fuse.js, vercel/ms, con benchmark
- [Installare, aggiornare e sviluppare](install.md): ogni metodo di installazione, Windows e la compilazione dai sorgenti
- [FAQ](faq.md) · [Stato](STATUS.md) · [Strumenti MCP](tools.md)

## Sponsor

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> è libero e open source. Se aiuta il tuo lavoro, considera di sostenerlo.<br>
  Il tuo supporto tiene in vita lo sviluppo.
</p>

<p align="center">
  <a href="https://coderabbit.ai">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="Sponsor aziendale: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="Sponsor: Santosh Yadav, Hakan Soytürk, Ali Yilmaz, Ashley Hunter, Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/Become_a_sponsor-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="Become a sponsor"></a>
  <a href="https://buymeacoffee.com/erkamyaman"><img src="https://img.shields.io/badge/Buy_me_a_coffee-%E2%98%95-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee"></a>
</p>

## Contribuire

Issue e pull request sono benvenute; [CONTRIBUTING.md](../CONTRIBUTING.md) copre la configurazione,
dove vanno i test e come aggiungere un linguaggio. Le scelte di progetto e le loro ragioni sono in
[DECISIONS.md](../DECISIONS.md). I casi di studio di port reali sono particolarmente graditi. Segnala
privatamente i problemi di sicurezza ([SECURITY.md](../SECURITY.md)) e consulta il
[codice di condotta](../CODE_OF_CONDUCT.md).

## Licenza

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). I port e il codice generato nel tuo repository sono tuoi.