<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="El cangrejo de rust-anything">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a> · Español · <a href="README_IT.md">Italiano</a></sub></p>
<p align="center"><sub>Esta traducción la mantiene la comunidad y puede quedarse atrás respecto al <a href="../README.md">README en inglés</a>.</sub></p>

<p align="center"><b>Reescribe tu aplicación en Rust y demuestra que sigue comportándose igual.</b><br>
Apúntalo a tu aplicación: registra lo que hace el original, tu agente de programación escribe el Rust pieza a pieza,<br>
y cada pieza se comprueba contra ese registro antes de pasar a la siguiente. No traduce código: demuestra la reescritura.</p>

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

<p align="center"><b><a href="#inicio-rápido">Instalar</a></b> · <a href="https://github.com/erkamyaman/rust-anything/releases">Versiones</a> · <a href="#inicio-rápido">Plugin para Claude Code</a> · <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=rust-anything">Registro MCP</a> · <a href="real-world.md">Portes reales</a> · <a href="faq.md#faq">Preguntas frecuentes</a><br>
<sub><a href="#lenguajes">Lenguajes</a> · <a href="how-it-works.md#how-it-works">Cómo funciona</a> · <a href="whole-apps.md#port-a-whole-app">Aplicaciones completas</a> · <a href="#patrocinadores">Patrocinadores</a> · <a href="../LICENSE">MIT</a></sub><br>
Hecho en Türkiye 🇹🇷 por <a href="https://github.com/erkamyaman">erKam Yaman</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ Patrocíname</a> · Con el patrocinio de <a href="https://coderabbit.link/erkamyaman"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center">Nuevo en 2.0: <a href="real-world.md#prettier">Prettier, portado a Rust</a>. 19 veces más rápido · 15.861 de 17.224 ejecuciones registradas enteramente en Rust · cada ejecución idéntica byte a byte</p>

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier y su porte en Rust formatean los mismos 59 archivos reales, un proceso por archivo: Prettier tarda 3,63 s y el porte en Rust 0,20 s, con la misma salida byte a byte." width="820">
  <br>
  <sub>Prettier frente a su porte en Rust sobre 59 archivos reales de ms, Fuse.js y json-server (Apple M4). La misma salida, byte a byte. <a href="real-world.md#prettier">Cómo se midió</a>.</sub>
</p>

<p align="center"><sub>PHP y C#: hoy registro, repetición y puerta delantera; los portes de unidad vienen después.</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>Registrar</b><br>
      <code>record</code> ejecuta el original con casos límite generados, trampas semánticas y tus propias entradas, y guarda lo que devuelve o lanza.
    </td>
    <td width="25%" valign="top">
      <b>Portar</b><br>
      <code>bridge</code> genera el crate de Rust y un envoltorio, para que quien llama conserve la API original. El Rust lo escribes tú o tu agente.
    </td>
    <td width="25%" valign="top">
      <b>Comprobar</b><br>
      <code>diff</code> repite cada caso registrado contra el Rust, y nada se publica hasta que todos coinciden y <code>gate</code> pasa.
    </td>
    <td width="25%" valign="top">
      <b>Elevar</b><br>
      Cuando la repetición pasa sin recurrir al original y cada unidad pasa gate, <code>app lift</code> lo convierte en una aplicación de Rust pura. El original nunca se toca.
    </td>
  </tr>
</table>

**Reescribe toda tu aplicación en Rust, una pieza a la vez.** rust-anything registra lo que hoy hace
tu aplicación de Node.js (TypeScript/JavaScript), Python, C, Go, Ruby, Java o Kotlin, y después
comprueba cada pieza de la reescritura en Rust contra ese registro antes de pasar a la siguiente,
hasta que la aplicación es Rust puro. CLIs, servidores HTTP y bibliotecas; el Rust lo escribe tu
agente de programación (o tú). Se apoya en [napi-rs](https://github.com/napi-rs/napi-rs), [PyO3](https://github.com/PyO3/pyo3) y [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen), que llevan las llamadas entre tu lenguaje y Rust; rust-anything decide qué mover y demuestra que coincide. Solo puede detectar lo que cubre el registro: consulta
[Qué no puede detectar](how-it-works.md#what-it-cant-catch). Herramientas y servidores completos, o
solo la parte lenta, una ruta caliente a la vez:

| Proyecto real, misma salida que el original | Original | Con rust-anything |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2, 2.360 archivos JS/TS desde la línea de comandos | 128,6 s | **6,9 s: 19 veces más rápido** |
| Prettier, 6.305 archivos Markdown desde la línea de comandos | 352 s | **22,5 s: 15,6 veces más rápido** |
| [json-server](real-world.md#json-server), un servidor Node completo: memoria en reposo | 74 MB | **3,5 MB**, un binario de 5,3 MB |
| json-server, del arranque a la primera respuesta | 79 a 85 ms | **8 a 10 ms** |
| [Fuse.js](real-world.md#fusejs) búsqueda sobre 50.000 cadenas, por consulta | 91,2 ms | **2,8 ms: 32 veces más rápido** |

Prettier: 15.861 de 17.224 ejecuciones registradas formatean enteramente en Rust en 10 lenguajes
(JavaScript, TypeScript, JSX, JSON, GraphQL, YAML, Markdown, CSS, SCSS, Less), y las 17.224
coinciden con Prettier byte a byte, porque todo lo que aún no está portado recurre a Prettier.
json-server: 514 de 514 casos HTTP registrados coinciden en un binario de Rust puro. Medido en un
Apple M4, dos ejecuciones de cada uno, comparando cada salida antes de cronometrar
([detalles](real-world.md)). Llamar a Rust una vez por función pequeña suele ser *más lento* que un
JIT; rust-anything encuentra el bucle que merece la pena mover, y lo mueve. Frente a C la ventaja es
la seguridad, no la velocidad: [libavutil de ffmpeg](real-world.md#ffmpeg) portado a Rust coincide en
las 226.447 llamadas registradas y corre a la velocidad de C (18 de 42 benchmarks iguales, 8 más
rápidos y 16 más lentos entre un 3 y un 40 %).

## Solo la parte lenta

No tienes que portar toda la aplicación. Apunta `go` a un comando demasiado lento, y porta la única
función que se lleva el tiempo:

```sh
rust-anything go --agent claude -- python3 app.py
```

Perfila la ejecución, elige la función más externa que merece la pena mover (el bucle, no la pequeña
llamada de dentro, por lotes donde el perfil lo indica), la registra con entradas generadas más las
llamadas que hace tu ejecución, y entrega `src/port.rs` a tu agente (`claude`, `codex` o cualquier
comando que lea un prompt por stdin; sin `--agent` escribe `TASK.md` por ti y se detiene). Solo activa
la función cuando cada caso registrado coincide y `gate` pasa. Después:

```sh
rust-anything report rust/dedupe-rs     # REPORT.md y card.svg para el pull request
rust-anything disable rust/dedupe-rs    # de vuelta al original, byte a byte
```

La fila de [Fuse.js](real-world.md#fusejs) de arriba tiene esta forma: la puntuación de Fuse para una
cadena, por lotes sobre la lista, se movió a Rust mientras las claves de objeto, la búsqueda extendida
y el resto siguen en JavaScript, de 91,2 ms a 2,8 ms por consulta sobre 50.000 cadenas. Prueba `go` en
[`tests/fixtures/go-fuzzy`](../tests/fixtures/go-fuzzy), un dedupe de Python de 20 líneas. Todas las formas
de activar y desactivar un porte: [Switch a port on and off](how-it-works.md#switch-a-port-on-and-off).

<p align="center">
  <img src="assets/json-server.gif" alt="json-server como servidor Node y como un único binario Rust: memoria en reposo 74 MB y 3,5 MB, del arranque a la primera respuesta 79 a 85 ms y 8 a 10 ms, y 514 de 514 casos HTTP registrados coinciden." width="820">
  <br>
  <sub>json-server, un servidor Node completo, elevado a un binario de Rust puro (Apple M4). <a href="real-world.md#json-server">Cómo se midió</a>.</sub>
</p>

> rust-anything es un proyecto independiente; no está afiliado ni respaldado por el Rust Project ni
> por la Rust Foundation. «Rust» es una marca de la Rust Foundation.

> [!WARNING]
> `record` y `app record` ejecutan tu código original, llamando a cada función (o a la aplicación
> entera) con muchas entradas generadas, y `app record` ejecuta cada caso dos veces. `diff`, `gate`
> y `app replay` vuelven a ejecutar los mismos casos, sobre el porte o sobre el original. El código
> que escribe archivos, habla con una base de datos o llama a la red lo hace una vez por caso.
> Ejecútalos sobre una copia del proyecto, con una base de datos de pruebas y sin credenciales de
> producción en el entorno.

## Instalar los portes

El porte de ESLint es un comando npm. Ejecuta el porte en Rust para lo que cubre y ESLint 10.15.0,
fijado como dependencia, para todo lo demás, así que la salida es la de ESLint en cualquier caso:

```sh
npm install -g eslint-rs     # ESLint 10.15.0
```

| Comando | Respondido en Rust | Cedido a ESLint 10.15.0 |
| --- | --- | --- |
| `eslint-rs` | `--no-config-lookup --rule ... --stdin` con las 90 reglas portadas (salida `stylish` y `json`, arreglos, sugerencias) | búsquedas de `eslint.config.*` (`eslint .`), archivos y carpetas, las otras 202 reglas del núcleo, plugins, parsers, los demás formateadores, errores de análisis |

El binario existe para macOS (arm64, x64) y Linux (x64, arm64, glibc). En Windows y en el resto, o con
`RUST_ANYTHING_MODE=old`, `eslint-rs` ejecuta ESLint: la misma salida, sin ganancia de velocidad.
Instalado desde sus tarballs en macOS arm64, `eslint-rs` coincide con ESLint 10.15.0 instalado al
lado en las 10.585 ejecuciones registradas ([cómo](packaging.md#a-whole-cli-front-door)).

El porte de Prettier se empaqueta igual (`prettier-rs`, Prettier 3.6.2) y repite 17.224 de 17.224
ejecuciones registradas desde sus tarballs, con un 7,9 % cedido a Prettier, pero no está en npm.
`rust-anything package` produce este tipo de paquete desde cualquier puerta delantera de CLI.

## Inicio rápido

Pega esto en Claude Code, Codex, Cursor, Gemini CLI u OpenCode y deja que tu agente lo instale:

```text
Install rust-anything: run `cargo install rust-anything`, then `rust-anything setup`.
Tell me when to restart you.
```

O hazlo tú:

```sh
cargo install rust-anything   # desde crates.io
rust-anything setup           # lo registra en Claude Code, Codex, Cursor, Gemini CLI y OpenCode
```

¿Todavía sin toolchain de Rust? Los binarios ya compilados para macOS, Linux y Windows van adjuntos
a cada [versión de GitHub](https://github.com/erkamyaman/rust-anything/releases), y para proyectos
JavaScript `npm install -g rust-anything` instala el mismo binario.

En Claude Code también puedes instalarlo como plugin (el servidor MCP y la skill; la CLI anterior
tiene que estar en tu `PATH`):

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

En Codex, desde el marketplace de agentes de este repositorio
(`.agents/plugins/marketplace.json`, el mismo plugin bajo `plugin/`, con la skill y el servidor MCP):

```sh
codex plugin marketplace add erkamyaman/rust-anything
codex plugin add rust-anything@rust-anything
```

`setup` añade el servidor MCP a cada agente que encuentra e instala la skill del flujo de trabajo
donde el agente lee skills. Conserva tus demás ajustes, se puede ejecutar otra vez sin problema y
`--remove` lo deshace. Después pídeselo a tu agente, por ejemplo:

- *"Reescribe esta CLI en Rust con rust-anything, de principio a fin, y demuestra que se comporta igual."*
- *"Esta búsqueda va lenta. Perfílala con rust-anything y mueve el bucle caliente a Rust."*
- *"Planifica una migración de `src/parser/` a Rust con rust-anything; porta la primera unidad y demuéstralo."*
- *"Compara el porte en Rust de `rust/text-rs` con el original y dime si merece la pena publicarlo."*

### Funciona con

| Agente | Lo que añade `rust-anything setup` |
| --- | --- |
| Claude Code | servidor MCP en `~/.claude.json`, skill en `~/.claude/skills/` |
| Codex | servidor MCP en `~/.codex/config.toml`, skill en `~/.codex/skills/` |
| Cursor | servidor MCP en `~/.cursor/mcp.json` |
| Gemini CLI | servidor MCP en `~/.gemini/settings.json` |
| OpenCode | servidor MCP en `~/.config/opencode/opencode.json` (`mcp.servers`), skill en `~/.config/opencode/skills/` |
| Cualquier otro cliente MCP | añade `rust-anything mcp` como servidor stdio a mano (ver [Instalar](install.md#install)) |

Antes de cambiar un archivo que ya existe, `setup` guarda una copia junto a él
(`<file>.rust-anything.bak`).

Para configurar OpenCode a mano, pon el servidor en `~/.config/opencode/opencode.json` (`command` es
el argv completo) y copia [`skills/rust-anything/`](../skills/rust-anything) a
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

O ejecuta los pasos tú mismo:

```sh
rust-anything profile . -- node bench.mjs              # qué está caliente, y quién lo llama
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# escribe src/port.rs (o deja que lo haga tu agente), y después:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # cada caso registrado debe coincidir
rust-anything fuzz rust/fuzzy-rs --time 60 --keep      # entradas nuevas: lo que el registro no cubrió
rust-anything bench rust/fuzzy-rs                      # ¿más rápido? el veredicto lo dice
rust-anything gate rust/fuzzy-rs                       # terminado significa que esto pasa
```

rust-anything no traduce código: el Rust lo escriben tu agente de programación (Claude Code, Codex,
Cursor, OpenCode, ...) o tú. rust-anything aporta lo que un agente no puede adivinar con fiabilidad:
dónde se va el tiempo, qué portar primero y en qué orden, un puente para que el resto del código
llame al Rust con su API intacta, el comportamiento exacto que el porte debe igualar, una forma
segura de publicarlo y una compuerta que dice cuándo un porte está terminado.

La misma salida no basta. Registra también lo que envía la aplicación, y una reescritura que manda
el correo dos veces falla:

<p align="center">
  <img src="assets/effects.gif" alt="app record con --effects smtp registra un correo de bienvenida por alta; app replay de una reescritura que imprime la misma salida pero envía el correo dos veces informa 0/2 casos superados, falla por los efectos y el informe muestra el correo de más." width="820">
  <br>
  <sub>Una ejecución real de <code>app record --effects</code> y <code>app replay</code>. Ver <a href="whole-apps.md#side-effects-email-webhooks-and-databases">Efectos secundarios</a>.</sub>
</p>

## Lenguajes

| Lenguaje | Unidad | Puente | Registrado y repetido por |
| --- | --- | --- | --- |
| TypeScript/JavaScript | carpeta o archivo | napi-rs o wasm-bindgen con un cable binario generado; clases y async incluidos | Node, ejecutando los fuentes `.ts` |
| Python | paquete o módulo | PyO3 (abi3), conversión directa, excepciones tipadas | Python, importando el original |
| C | archivo `.c` | funciones `extern "C"` con los nombres originales y una cabecera | el original compilado a una biblioteca compartida, llamada por ctypes, un fork por caso |
| Go | paquete | llamadas cgo tipadas y un cable binario generado, en un paquete con la API original | un driver de Go generado, compilado como su propio módulo |
| Ruby | archivo o carpeta de funciones de módulo | una extensión magnus bajo los módulos originales | un driver de Ruby generado |
| Java | paquete de métodos `public static` | JNI tipado y un cable binario generado, en clases envoltorio con los métodos originales | un driver de reflexión generado |
| Kotlin | módulo Gradle de funciones de nivel superior y métodos `@JvmStatic` | el puente de Java: JNI tipado en clases envoltorio de Java que llaman a los estáticos de Kotlin compilados | un driver de reflexión generado, con los fuentes Kotlin compilados por `kotlinc` |
| PHP | carpeta de funciones y métodos `public static` | lo siguiente: FFI de PHP sobre una ABI de C | hoy aplicaciones completas: `app record`, `app replay` y la puerta delantera, para CLIs y servidores (`php -S`) |
| C# | carpeta de métodos `public static` | lo siguiente: P/Invoke sobre una ABI de C | hoy aplicaciones completas: `app record`, `app replay` y la puerta delantera, para CLIs y servidores ASP.NET (`dotnet <app>.dll`) |

Todos los lenguajes salvo C reciben además clases por lotes (`--batch`): los elementos cruzan una
vez y se quedan en Rust, y cada llamada se ejecuta sobre todos ellos en todos los núcleos.

El lenguaje se detecta en el repositorio (`go.mod`, `pom.xml`, `build.gradle`/`build.gradle.kts`, un
`.csproj` o `.sln`, `pyproject.toml`, `Gemfile`, `composer.json`, `package.json`, o si no, los
archivos de código más comunes); un módulo Gradle con fuentes `.kt` y sin fuentes `.java` es Kotlin,
los módulos mixtos siguen siendo Java. `--lang` lo sobrescribe. Los tipos vienen del código: tipos
de TS, hints de Python, firmas de C, Go, Java, Kotlin y C#, etiquetas YARD de Ruby, tipos y phpdoc de
PHP; los parámetros sin tipo usados como números se infieren como números.

Para Android, `rust-anything build <crate> --profile android` compila la misma biblioteca JNI con el
NDK para `arm64-v8a`, `armeabi-v7a` y `x86_64` (o los valores de `--abi` que pases) y la instala en
`<crate>/jniLibs/<abi>/`; `--api` fija el nivel de API (mínimo 26, que es lo que necesita el runtime
de Java generado). Encuentra el NDK en `ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT` o en el SDK, necesita
los targets de Rust (`rustup target add aarch64-linux-android armv7-linux-androideabi
x86_64-linux-android`), enlaza las bibliotecas de 64 bits con alineación de página de 16 KB y
comprueba que cada punto de entrada JNI esté exportado. Gradle busca en `src/main/jniLibs` por
defecto, así que pasa `--jni-libs app/src/main/jniLibs` o añade `<crate>/jniLibs` a
`sourceSets.main.jniLibs.srcDirs`. `rust-anything record <crate> --profile android` ejecuta la
unidad Kotlin original bajo ART en un dispositivo conectado (adb), convierte a dex el driver
generado con `d8` de las build-tools del SDK y trae los resultados como el mismo `cases.jsonl`;
`diff <crate> --profile android` los repite contra el `.so` compilado para la ABI del dispositivo.
Ambos necesitan `adb`, un dispositivo, `kotlinc`, un SDK con build-tools y una plataforma, y
`--device <serial>` cuando hay más de un dispositivo conectado. El perfil cubre unidades de lógica:
la UI de Activity/Compose/XML, el ciclo de vida y las clases exclusivas del SDK se quedan en Kotlin,
y cada llamada cruza JNI, así que para bucles calientes conviene usar los envoltorios por lotes
generados. En un host, `record` y `diff` siguen usando la JVM de escritorio.

Para PHP, `analyze` planifica una aplicación PHP (paquetes de Composer y autoload PSR-4, riesgos
como la comparación laxa y el desbordamiento de enteros a coma flotante, paquetes de Composer
mapeados a crates), y una CLI o un servidor PHP se puede registrar, repetir y poner detrás de una
puerta delantera en Rust. Portar sus unidades a Rust mediante la FFI de PHP es el paso siguiente
([issue #2](https://github.com/erkamyaman/rust-anything/issues/2)).

C# funciona igual: `analyze` lee proyectos `.csproj` y sus paquetes NuGet (mapeados a crates),
resuelve las directivas `using` a carpetas y señala lo que C# hace distinto (`decimal`, `dynamic`,
las consultas diferidas de LINQ, `Task`, `lock`, reflexión). Una CLI de C# o un servidor ASP.NET se
puede registrar, repetir y poner detrás de una puerta delantera en Rust; portar unidades mediante
P/Invoke viene después.

## Saber más

- [Cómo funciona](how-it-works.md): el ciclo, lo que deciden `bridge` y `analyze`, rutas calientes, qué no puede detectar, qué cuesta un porte real y cómo seguir al original (`map`)
- [Publicar un porte como paquete](packaging.md): wheels de maturin y paquetes npm por plataforma de napi-rs, con el CI que los construye
- [Portar una aplicación completa](whole-apps.md): servidores, CLIs y bibliotecas, desde la puerta delantera hasta `app lift`
- [Ejemplos y casos de estudio](examples.md): x18n y código real en seis lenguajes
- [Portes reales](real-world.md): json-server, Fuse.js, vercel/ms, con benchmarks
- [Instalar, actualizar y desarrollar](install.md): todos los métodos de instalación, Windows y compilar desde el código fuente
- [Preguntas frecuentes](faq.md) · [Estado](STATUS.md) · [Herramientas MCP](tools.md)

## Patrocinadores

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> es libre y de código abierto. Si te ayuda en tu trabajo, considera patrocinarlo.<br>
  Tu apoyo mantiene el desarrollo en marcha.
</p>

<p align="center">
  <a href="https://coderabbit.ai">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="Patrocinador de la empresa: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="Patrocinadores: Santosh Yadav, Hakan Soytürk, Ali Yilmaz, Ashley Hunter, Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/Become_a_sponsor-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="Become a sponsor"></a>
  <a href="https://buymeacoffee.com/erkamyaman"><img src="https://img.shields.io/badge/Buy_me_a_coffee-%E2%98%95-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee"></a>
</p>

## Contribuir

Las issues y los pull requests son bienvenidos; [CONTRIBUTING.md](../CONTRIBUTING.md) cubre la
puesta en marcha, dónde van los tests y cómo añadir un lenguaje. Las decisiones de diseño y sus
razones están en [DECISIONS.md](../DECISIONS.md). Los casos de estudio de portes reales son
especialmente bienvenidos. Informa en privado de los problemas de seguridad
([SECURITY.md](../SECURITY.md)) y consulta el [código de conducta](../CODE_OF_CONDUCT.md).

## Licencia

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). Los portes y el código generado en tu repositorio son tuyos.
