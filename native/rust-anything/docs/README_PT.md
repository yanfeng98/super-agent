<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="O caranguejo do rust-anything">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · Português · <a href="README_VI.md">Tiếng Việt</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a></sub></p>
<p align="center"><sub>Esta tradução é mantida pela comunidade e pode estar desatualizada em relação ao <a href="../README.md">README em inglês</a>.</sub></p>

<p align="center"><b>Reescreva a sua aplicação em Rust e prove que continua a comportar-se da mesma forma.</b><br>
Não traduz código: o seu agente de programação (ou você) escreve o Rust, e o rust-anything verifica-o contra o original.</p>

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

<p align="center"><b><a href="#início-rápido">Início rápido</a></b> · <a href="https://github.com/erkamyaman/rust-anything/releases">Releases</a> · <a href="#início-rápido">Claude Code plugin</a> · <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=rust-anything">MCP Registry</a> · <a href="real-world.md">Migrações reais</a> · <a href="faq.md#faq">Perguntas frequentes</a><br>
<sub><a href="#linguagens">Linguagens</a> · <a href="how-it-works.md#how-it-works">Como funciona</a> · <a href="whole-apps.md#port-a-whole-app">Aplicações completas</a> · <a href="../LICENSE">MIT</a></sub><br>
Feito na Turquia 🇹🇷 por <a href="https://github.com/erkamyaman">erKam Yaman</a> · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ Patrocine-me</a> · Patrocinado por <a href="https://coderabbit.link/erkamyaman"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center">Novo na 2.0: <a href="real-world.md#prettier">Prettier, migrado para Rust</a>. 19x mais rápido · 15,861 de 17,224 execuções registadas inteiramente em Rust · todas idênticas byte a byte</p>

<p align="center">
  <img src="assets/prettier.gif" alt="O Prettier e a sua migração para Rust formatam os mesmos 59 ficheiros reais, um processo por ficheiro: o Prettier demora 3.63 s, a migração para Rust 0.20 s, com a mesma saída byte a byte." width="820">
  <br>
  <sub>O Prettier contra a sua migração para Rust em 59 ficheiros reais do ms, Fuse.js e json-server (Apple M4). A mesma saída, byte a byte. <a href="real-world.md#prettier">Como foi medido</a>.</sub>
</p>

<p align="center"><sub>PHP e C#: gravação, repetição e front door hoje; a migração de unidades vem a seguir.</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>Registar</b><br>
      <code>record</code> executa o original sobre casos limite gerados, armadilhas semânticas e as suas próprias entradas, e guarda o que ele devolve ou lança.
    </td>
    <td width="25%" valign="top">
      <b>Migrar</b><br>
      <code>bridge</code> gera o crate Rust e um wrapper, para que quem chama mantenha a API original. Você ou o seu agente escrevem o Rust.
    </td>
    <td width="25%" valign="top">
      <b>Verificar</b><br>
      <code>diff</code> reproduz cada caso registado contra o Rust, e nada é publicado até todos coincidirem e o <code>gate</code> passar.
    </td>
    <td width="25%" valign="top">
      <b>Lift</b><br>
      Quando o replay passa sem fallback e todas as unidades passam o gate, <code>app lift</code> transforma-a numa aplicação em Rust puro. O original nunca é alterado.
    </td>
  </tr>
</table>

**Reescreva toda a sua aplicação em Rust, uma peça de cada vez.** O rust-anything regista o comportamento atual da sua aplicação Node.js (TypeScript/JavaScript), Python, C, Go, Ruby ou Java e, em seguida, compara cada parte da reescrita em Rust com esse registo antes de avançar, até que a aplicação seja totalmente escrita em Rust. CLIs, servidores HTTP e bibliotecas; o seu agente de programação (ou você) escreve o código Rust. A ferramenta só consegue detetar aquilo que o registo abrange: consulte [O que não consegue detetar](how-it-works.md#what-it-cant-catch). Ferramentas e servidores completos, ou apenas a parte lenta, um caminho crítico de cada vez:

| Projeto real, mesma saída do original | Original | Com rust-anything |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2, 2,360 ficheiros JS/TS na linha de comandos | 128.6 s | **6.9 s: 19x mais rápido** |
| Prettier, 6,305 ficheiros Markdown na linha de comandos | 352 s | **22.5 s: 15.6x mais rápido** |
| [json-server](real-world.md#json-server), um servidor Node completo: memória em repouso | 74 MB | **3.5 MB**, um único binário de 5.3 MB |
| json-server, do arranque à primeira resposta | 79 a 85 ms | **8 a 10 ms** |
| Pesquisa [Fuse.js](real-world.md#fusejs) em 50,000 strings, por consulta | 91.2 ms | **2.8 ms: 32x mais rápido** |

Prettier: 15,861 de 17,224 execuções registadas são formatadas totalmente em Rust, em 10 linguagens (JavaScript, TypeScript, JSX, JSON, GraphQL, YAML, Markdown, CSS, SCSS, Less), e todas as 17,224 coincidem com o Prettier byte a byte, já que tudo o que ainda não foi migrado recorre ao Prettier. json-server: 514/514 casos HTTP registados coincidem num binário em Rust puro. Medições num Apple M4, duas execuções cada, com cada saída comparada antes da medição de tempo ([detalhes](real-world.md)). Chamar Rust uma vez por cada pequena função costuma ser *mais lento* do que utilizar um JIT; o rust-anything identifica o ciclo que realmente vale a pena migrar e transfere-o.

## Apenas a parte lenta

Não precisa de migrar a aplicação inteira. Aponte o `go` para um comando demasiado lento, e ele migra apenas a função que consome o tempo:

```sh
rust-anything go --agent claude -- python3 app.py
```

Analisa o perfil da execução, escolhe a função mais exterior que vale a pena mover (o ciclo, não a pequena chamada dentro dele, em lote quando o perfil o indica), regista-a sobre entradas geradas e sobre as chamadas que a sua execução faz, e entrega `src/port.rs` ao seu agente (`claude`, `codex` ou qualquer comando que leia um prompt do stdin; sem `--agent`, escreve `TASK.md` para si e termina aí). Só ativa a função quando todos os casos registados coincidem e o `gate` passa. Depois:

```sh
rust-anything report rust/dedupe-rs     # REPORT.md e card.svg para o pull request
rust-anything disable rust/dedupe-rs    # de volta ao original, byte a byte
```

A linha [Fuse.js](real-world.md#fusejs) acima tem esta forma: a pontuação do Fuse para uma string, processada em lote sobre a lista, foi movida para Rust, enquanto as chaves de objeto, a pesquisa estendida e o resto ficam em JavaScript, passando de 91.2 ms para 2.8 ms por consulta em 50,000 strings. Experimente o `go` em [`tests/fixtures/go-fuzzy`](../tests/fixtures/go-fuzzy), uma deduplicação em Python com 20 linhas. Todas as formas de ativar e desativar uma migração: [Switch a port on and off](how-it-works.md#switch-a-port-on-and-off).

<p align="center">
  <img src="assets/json-server.gif" alt="json-server como servidor Node e como um único binário Rust: memória em repouso 74 MB e 3.5 MB, do arranque à primeira resposta 79 a 85 ms e 8 a 10 ms, e 514 de 514 casos HTTP registados coincidem." width="820">
  <br>
  <sub>json-server, um servidor Node completo, transformado num binário em Rust puro (Apple M4). <a href="real-world.md#json-server">Como foi medido</a>.</sub>
</p>

> O rust-anything é um projeto independente, sem qualquer afiliação ou aprovação do Rust Project ou da Rust Foundation. "Rust" é uma marca registada da Rust Foundation.

> [!WARNING]
> `record` e `app record` executam o seu código original, chamando cada função (ou a aplicação completa) com
> muitas entradas geradas, e `app record` executa cada caso duas vezes. `diff`, `gate` e `app replay`
> voltam a executar os mesmos casos, na migração ou no original. Código que escreve ficheiros, comunica com uma
> base de dados ou acede à rede fá-lo uma vez por caso. Execute-os numa cópia do projeto, com uma
> base de dados de teste e sem credenciais de produção no ambiente.

## Início rápido

Cole isto no Claude Code, Codex, Cursor, Gemini CLI ou OpenCode e deixe o seu agente instalá-lo:

```text
Install rust-anything: run `cargo install rust-anything`, then `rust-anything setup`.
Tell me when to restart you.
```

Ou faça a instalação manualmente:

```sh
cargo install rust-anything   # a partir do crates.io
rust-anything setup           # regista a ferramenta no Claude Code, Codex, Cursor, Gemini CLI e OpenCode
```

Ainda não tem a toolchain do Rust? Os binários pré-compilados para macOS, Linux e Windows estão disponíveis em cada [lançamento no GitHub](https://github.com/erkamyaman/rust-anything/releases). Para projetos JavaScript, `npm install -g rust-anything` instala o mesmo binário.

No Claude Code, também pode instalá-lo como plugin (o servidor MCP e a skill; a CLI acima deve estar disponível no seu `PATH`):

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

O comando `setup` adiciona o servidor MCP a cada agente que encontrar e instala a skill do fluxo de trabalho no local onde o agente procura as suas skills. Preserva as suas outras configurações, pode ser executado novamente em segurança e `--remove` desfaz as alterações. Depois, peça ao seu agente algo como:

- *"Reescreve esta CLI em Rust com rust-anything, de ponta a ponta, e comprova que se comporta da mesma forma."*
- *"Esta pesquisa parece lenta. Analisa o desempenho com rust-anything e transfere o ciclo mais pesado para Rust."*
- *"Planeia a migração de `src/parser/` para Rust com rust-anything; migra a primeira unidade e comprova que funciona."*
- *"Compara o port Rust em `rust/text-rs` com o original e diz-me se está pronto para produção."*

### Compatibilidade com agentes

| Agente | O que `rust-anything setup` adiciona |
| --- | --- |
| Claude Code | Servidor MCP em `~/.claude.json`, skill em `~/.claude/skills/` |
| Codex | Servidor MCP em `~/.codex/config.toml`, skill em `~/.codex/skills/` |
| Cursor | Servidor MCP em `~/.cursor/mcp.json` |
| Gemini CLI | Servidor MCP em `~/.gemini/settings.json` |
| OpenCode | Servidor MCP em `~/.config/opencode/opencode.json` (`mcp.servers`), skill em `~/.config/opencode/skills/` |
| Qualquer outro cliente MCP | Adicione manualmente `rust-anything mcp` como servidor stdio (consulte [Instalação](install.md#install)) |

Antes de modificar um ficheiro existente, `setup` guarda uma cópia ao lado dele (`<file>.rust-anything.bak`).

Para configurar o OpenCode manualmente, coloque o servidor em `~/.config/opencode/opencode.json` (`command` é o argv completo) e copie [`skills/rust-anything/`](../skills/rust-anything) para `~/.config/opencode/skills/rust-anything/`:

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

Ou execute as etapas manualmente:

```sh
rust-anything profile . -- node bench.mjs              # o que está lento e quem o chama
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# escreva src/port.rs (ou deixe o seu agente fazê-lo) e depois:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # todos os casos registados devem corresponder
rust-anything bench rust/fuzzy-rs                      # está mais rápido? o resultado dirá
rust-anything gate rust/fuzzy-rs                       # concluído significa que este teste passa
```

O rust-anything não traduz código: o seu agente de programação (Claude Code, Codex, Cursor, OpenCode, ...) ou você escreve o código Rust. O rust-anything fornece aquilo que um agente não consegue adivinhar de forma fiável: onde o tempo é gasto, o que deve ser migrado primeiro e em que ordem, uma ponte para que o restante código continue a chamar Rust com a mesma API, o comportamento exato que a migração deve preservar, uma forma segura de disponibilizar a nova implementação e uma verificação que determina quando a migração está concluída.

A mesma saída não basta. Registe também o que a aplicação envia, e uma reescrita que envia o e-mail duas vezes falha:

<p align="center">
  <img src="assets/effects.gif" alt="app record com --effects smtp regista um e-mail de boas-vindas por registo de utilizador; app replay de uma reescrita que imprime a mesma saída mas envia o e-mail duas vezes indica 0/2 casos aprovados, com falha nos efeitos, e o relatório mostra o e-mail a mais." width="820">
  <br>
  <sub>Uma execução real de <code>app record --effects</code> e <code>app replay</code>. Consulte <a href="whole-apps.md#side-effects-email-webhooks-and-databases">Efeitos secundários</a>.</sub>
</p>

## Linguagens

| Linguagem | Unidade | Ponte | Registado e reproduzido por |
| --- | --- | --- | --- |
| TypeScript/JavaScript | Pasta ou ficheiro | napi-rs ou wasm-bindgen com um protocolo binário gerado; inclui classes e operações assíncronas | Node, executando diretamente os ficheiros `.ts` |
| Python | Pacote ou módulo | PyO3 (abi3), conversão direta, exceções tipadas | Python, importando o original |
| C | Ficheiro `.c` | Funções `extern "C"` com os nomes originais e um header | O original compilado como biblioteca partilhada, chamado através de ctypes, uma execução independente por caso |
| Go | Pacote | Chamadas cgo tipadas e um protocolo binário gerado, num pacote com a API original | Um driver Go gerado, compilado como módulo independente |
| Ruby | Ficheiro ou pasta de funções de módulo | Uma extensão magnus nos módulos originais | Um driver Ruby gerado |
| Java | Pacote de métodos `public static` | JNI tipado e um protocolo binário gerado, em classes wrapper com os métodos originais | Um driver de reflexão gerado |
| PHP | Pasta de funções e métodos `public static` | Próximo passo: PHP FFI através de uma ABI C | Aplicações completas atualmente: `app record`, `app replay` e a front door, para CLIs e servidores (`php -S`) |
| C# | Pasta de métodos `public static` | Próximo passo: P/Invoke através de uma ABI C | Aplicações completas atualmente: `app record`, `app replay` e a front door, para CLIs e servidores ASP.NET (`dotnet <app>.dll`) |

Todas as linguagens, exceto C, também recebem classes de processamento em lote (`--batch`): os elementos atravessam a ponte uma única vez e permanecem em Rust, e cada chamada processa todos eles em todos os núcleos.

A linguagem é detetada a partir do repositório (`go.mod`, `pom.xml`, um `.csproj` ou `.sln`, `pyproject.toml`, `Gemfile`, `composer.json`, `package.json` ou, caso contrário, os ficheiros de código-fonte mais comuns). A opção `--lang` permite substituir essa deteção. Os tipos são obtidos a partir do código-fonte: tipos de TS, anotações de Python, assinaturas de C, Go, Java e C#, tags YARD de Ruby, tipos PHP e phpdoc. Parâmetros sem tipo que são utilizados como números são inferidos como números.

Para PHP, `analyze` planeia uma aplicação PHP (pacotes Composer e carregamento automático PSR-4, riscos como comparações flexíveis e overflow de inteiros para floats, pacotes Composer associados a crates), e uma CLI ou servidor PHP pode ser registado, reproduzido e colocado atrás de uma front door em Rust. A migração das suas unidades para Rust através do FFI do PHP é o próximo passo ([issue #2](https://github.com/erkamyaman/rust-anything/issues/2)).

O C# funciona da mesma forma: `analyze` lê projetos `.csproj` e os respetivos pacotes NuGet (associados a crates), resolve diretivas `using` para pastas e identifica diferenças do C# (`decimal`, `dynamic`, consultas adiadas do LINQ, `Task`, `lock` e reflexão). Uma CLI C# ou um servidor ASP.NET pode ser registado, reproduzido e colocado atrás de uma front door em Rust; a migração de unidades através de P/Invoke vem a seguir.

## Saber mais

- [Como funciona](how-it-works.md): o ciclo, o que `bridge` e `analyze` decidem, caminhos críticos, o que não consegue detetar, o que uma migração real exige e como acompanhar o original (`map`)
- [Migrar uma aplicação completa](whole-apps.md): servidores, CLIs e bibliotecas, da front door até ao `app lift`
- [Exemplos e estudos de caso](examples.md): x18n e código real em seis linguagens
- [Migrações reais](real-world.md): json-server, Fuse.js, vercel/ms, com benchmarks
- [Instalar, atualizar e desenvolver](install.md): todos os métodos de instalação, Windows e compilação a partir do código-fonte
- [Perguntas frequentes](faq.md) · [Estado](STATUS.md) · [Ferramentas MCP](tools.md)

## Patrocinadores

<p align="center">
  O <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> é gratuito e de código aberto. Se for útil para o seu trabalho, considere patrociná-lo.<br>
  O seu apoio ajuda a manter o desenvolvimento.
</p>

<p align="center">
  <a href="https://coderabbit.link/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="Patrocinador empresarial: CodeRabbit">
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
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/Tornar--se_patrocinador-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="Tornar-se patrocinador"></a>
  <a href="https://buymeacoffee.com/erkamyaman"><img src="https://img.shields.io/badge/Buy_me_a_coffee-%E2%98%95-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee"></a>
</p>

## Contribuir

Issues e pull requests são bem-vindos; [CONTRIBUTING.md](../CONTRIBUTING.md) explica a configuração, onde colocar os testes e como adicionar uma linguagem. As decisões de design e as respetivas justificações estão em [DECISIONS.md](../DECISIONS.md). Estudos de caso de migrações reais são especialmente bem-vindos. Por favor, comunique problemas de segurança de forma privada ([SECURITY.md](../SECURITY.md)) e consulte o [código de conduta](../CODE_OF_CONDUCT.md).

## Licença

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). As migrações e o código gerado no seu repositório pertencem-lhe.
