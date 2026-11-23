<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="rust-anything のカニ">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · 日本語 · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a></sub></p>
<p align="center"><sub>この翻訳はコミュニティによって管理されており、<a href="../README.md">英語版 README</a> より古い場合があります。</sub></p>

<p align="center"><b>アプリを Rust で書き直し、元と同じように動くことを証明する。</b><br>
コードを自動翻訳するツールではありません。Rust はコーディングエージェント（またはあなた）が書き、rust-anything が元のプログラムと照合して検証します。</p>

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

<p align="center"><b><a href="#クイックスタート">クイックスタート</a></b> · <a href="https://github.com/erkamyaman/rust-anything/releases">Releases</a> · <a href="#クイックスタート">Claude Code plugin</a> · <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=rust-anything">MCP Registry</a> · <a href="real-world.md">実プロジェクトの移植</a> · <a href="faq.md#faq">よくある質問</a><br>
<sub><a href="#対応言語">対応言語</a> · <a href="how-it-works.md#how-it-works">仕組み</a> · <a href="whole-apps.md#port-a-whole-app">アプリ全体の移植</a> · <a href="../LICENSE">MIT</a></sub><br>
トルコ 🇹🇷 で <a href="https://github.com/erkamyaman">erKam Yaman</a> が開発 · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ スポンサーになる</a> · スポンサー: <a href="https://coderabbit.link/erkamyaman"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center">2.0 の新機能: <a href="real-world.md#prettier">Prettier を Rust に移植</a>。19x 高速 · 記録済み実行 17,224 件中 15,861 件が完全に Rust · すべてバイト単位で一致</p>

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier とその Rust 移植版が、ファイルごとに 1 プロセスで同じ 59 個の実ファイルをフォーマット: Prettier は 3.63 s、Rust 移植版は 0.20 s で、出力はバイト単位で同一です。" width="820">
  <br>
  <sub>ms、Fuse.js、json-server の 59 個の実ファイルで Prettier と Rust 移植版を比較（Apple M4）。出力はバイト単位で同一です。<a href="real-world.md#prettier">計測方法</a>。</sub>
</p>

<p align="center"><sub>PHP と C#：現在は記録・再生と front door に対応、ユニット単位の移植は次の予定です。</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>記録</b><br>
      <code>record</code> は生成したエッジケース、セマンティックトラップ、あなた自身の入力で元の実装を実行し、返り値や例外を保存します。
    </td>
    <td width="25%" valign="top">
      <b>移植</b><br>
      <code>bridge</code> は Rust クレートとラッパーを生成するので、呼び出し元は元の API をそのまま使えます。Rust を書くのはあなたかエージェントです。
    </td>
    <td width="25%" valign="top">
      <b>検証</b><br>
      <code>diff</code> は記録したすべてのケースを Rust に対してリプレイします。すべて一致して <code>gate</code> に合格するまで何も出荷されません。
    </td>
    <td width="25%" valign="top">
      <b>リフト</b><br>
      フォールバックなしでリプレイに合格し、すべてのユニットが gate に合格すると、<code>app lift</code> がアプリを純粋な Rust アプリにします。元の実装には一切手を加えません。
    </td>
  </tr>
</table>

**アプリ全体を、一つずつ Rust に書き直しましょう。** rust-anything は、既存の Node.js（TypeScript/JavaScript）、Python、C、Go、Ruby、または Java アプリが現在どう振る舞うかを記録し、Rust で書き直した各パーツを次へ進む前にその記録と照合して、最終的にアプリ全体が純粋な Rust になるまで検証を続けます。CLI、HTTP サーバー、ライブラリに対応しており、Rust コードを書くのはコーディングエージェント（あるいはあなた自身）です。記録がカバーしている範囲しか検出できない点には注意してください（[検出できないもの](how-it-works.md#what-it-cant-catch)を参照）。ツールやサーバーを丸ごと、あるいは遅い部分だけを、ホットパス一つずつ移植できます:

| 実プロジェクト（元の実装と同一の出力） | 元の実装 | rust-anything 使用時 |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2、コマンドラインから 2,360 個の JS/TS ファイル | 128.6 s | **6.9 s: 19x 高速** |
| Prettier、コマンドラインから 6,305 個の Markdown ファイル | 352 s | **22.5 s: 15.6x 高速** |
| [json-server](real-world.md#json-server)、Node サーバー全体: アイドル時のメモリ | 74 MB | **3.5 MB**、5.3 MB のバイナリ 1 つ |
| json-server、起動から最初のレスポンスまで | 79〜85 ms | **8〜10 ms** |
| 50,000 文字列に対する [Fuse.js](real-world.md#fusejs) 検索、1 クエリあたり | 91.2 ms | **2.8 ms: 32x 高速** |

Prettier: 17,224 件の記録済み実行のうち 15,861 件が 10 言語（JavaScript、TypeScript、JSX、JSON、GraphQL、YAML、Markdown、CSS、SCSS、Less）にわたって完全に Rust でフォーマットされ、17,224 件すべてが Prettier とバイト単位で一致します。まだ移植されていない部分は Prettier にフォールバックするためです。json-server: 記録した 514/514 件の HTTP ケースが純粋な Rust バイナリ上で一致します。Apple M4 で各 2 回実行して計測し、計測前にすべての出力を比較しています（[詳細](real-world.md)）。小さな関数ごとに毎回 Rust を呼び出すと、通常は JIT より*遅く*なります。rust-anything は移植する価値のあるループを見つけ出し、そのループごと移行します。

## 遅い部分だけ

アプリ全体を移植する必要はありません。遅すぎるコマンドに `go` を向けると、時間を食っている関数を 1 つだけ移植します:

```sh
rust-anything go --agent claude -- python3 app.py
```

実行をプロファイリングし、移行する価値のある最も外側の関数を選びます（その中の小さな呼び出しではなくループを選び、プロファイルが示す場合はバッチ化します）。生成した入力と実行中に行われた呼び出しでその関数を記録し、`src/port.rs` をエージェントに渡します（`claude`、`codex`、または stdin からプロンプトを読む任意のコマンド。`--agent` を指定しない場合は `TASK.md` を書き出して停止します）。記録したすべてのケースが一致し、`gate` に合格した場合にのみ、その関数を有効にします。その後は次のとおりです:

```sh
rust-anything report rust/dedupe-rs     # プルリクエスト用の REPORT.md と card.svg
rust-anything disable rust/dedupe-rs    # 元の実装にバイト単位で戻す
```

上の表の [Fuse.js](real-world.md#fusejs) の行がこの形です。1 つの文字列に対する Fuse のスコア計算をリスト全体でバッチ化して Rust に移し、オブジェクトキー、拡張検索、その他の部分は JavaScript に残したまま、50,000 文字列に対して 1 クエリあたり 91.2 ms から 2.8 ms になりました。20 行の Python 製 dedupe である [`tests/fixtures/go-fuzzy`](../tests/fixtures/go-fuzzy) で `go` を試してみてください。移植版を有効・無効にするすべての方法: [Switch a port on and off](how-it-works.md#switch-a-port-on-and-off)。

<p align="center">
  <img src="assets/json-server.gif" alt="Node サーバーとしての json-server と単一の Rust バイナリとしての json-server: アイドル時のメモリは 74 MB と 3.5 MB、起動から最初のレスポンスまでは 79〜85 ms と 8〜10 ms、記録した 514 件中 514 件の HTTP ケースが一致します。" width="820">
  <br>
  <sub>Node サーバー全体である json-server を、純粋な Rust バイナリにリフト（Apple M4）。<a href="real-world.md#json-server">計測方法</a>。</sub>
</p>

> rust-anything は独立したプロジェクトであり、Rust Project または Rust Foundation と提携・承認関係にあるものではありません。"Rust" は Rust Foundation の商標です。

> [!WARNING]
> `record` と `app record` は元のコードを実行し、各関数（またはアプリ全体）を多数の生成入力で呼び出します。`app record` はすべてのケースを 2 回実行します。`diff`、`gate`、`app replay` は同じケースを移植版または元の実装で再度実行します。ファイルを書き込む、データベースにアクセスする、ネットワークを呼び出すコードは、ケースごとにその処理を行います。プロジェクトのコピー上で、テスト用データベースを使い、本番環境の認証情報が環境に含まれない状態で実行してください。

## クイックスタート

Claude Code、Codex、Cursor、Gemini CLI、または OpenCode に以下を貼り付けて、エージェントにインストールを任せてください:

```text
rust-anything をインストールして: `cargo install rust-anything` を実行し、次に `rust-anything setup` を実行してください。
あなたを再起動すべきタイミングが来たら教えてください。
```

または自分で実行します:

```sh
cargo install rust-anything   # crates.io からインストール
rust-anything setup           # Claude Code、Codex、Cursor、Gemini CLI、OpenCode に登録
```

Rust ツールチェーンをまだ導入していない場合は、各 [GitHub release](https://github.com/erkamyaman/rust-anything/releases) に macOS、Linux、Windows 向けのビルド済みバイナリが添付されています。JavaScript プロジェクトの場合は `npm install -g rust-anything` でも同じバイナリをインストールできます。

Claude Code ではプラグインとしてインストールすることもできます（MCP サーバーとワークフロースキルが含まれます。上記の CLI が `PATH` に通っている必要があります）:

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

`setup` は見つかった各エージェントに MCP サーバーを追加し、エージェントがスキルを読み込む場所にワークフロースキルをインストールします。他の設定は保持され、何度実行しても安全で、`--remove` で元に戻せます。あとはエージェントに次のように依頼するだけです:

- *"rust-anything を使ってこの CLI をエンドツーエンドで Rust に書き直し、元の実装と同じ挙動であることを証明して。"*
- *"この検索が遅く感じる。rust-anything でプロファイルを取って、ホットループを Rust に移して。"*
- *"rust-anything で `src/parser/` の Rust 移行計画を立てて。最初のユニットを移植して検証して。"*
- *"`rust/text-rs` の Rust 移植版と元の実装をベンチマーク比較して、出荷すべきか教えて。"*

### 対応エージェント

| エージェント | `rust-anything setup` が追加する内容 |
| --- | --- |
| Claude Code | `~/.claude.json` の MCP サーバー、`~/.claude/skills/` のスキル |
| Codex | `~/.codex/config.toml` の MCP サーバー、`~/.codex/skills/` のスキル |
| Cursor | `~/.cursor/mcp.json` の MCP サーバー |
| Gemini CLI | `~/.gemini/settings.json` の MCP サーバー |
| OpenCode | `~/.config/opencode/opencode.json` の MCP サーバー（`mcp.servers`）、`~/.config/opencode/skills/` のスキル |
| その他の MCP クライアント | 手動で `rust-anything mcp` を stdio サーバーとして追加（[インストール](install.md#install)を参照） |

既存のファイルを変更する前に、`setup` は同じ場所にバックアップ（`<file>.rust-anything.bak`）を保存します。

OpenCode を手動で設定するには、サーバーを `~/.config/opencode/opencode.json` に記述し（`command` は argv 全体です）、[`skills/rust-anything/`](../skills/rust-anything) を `~/.config/opencode/skills/rust-anything/` にコピーします:

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

自分で各ステップを実行することもできます:

```sh
rust-anything profile . -- node bench.mjs              # どこがホットスポットで、誰が呼んでいるか
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# src/port.rs を書く（またはエージェントに書かせる）。その後:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # 記録されたすべてのケースが一致する必要がある
rust-anything bench rust/fuzzy-rs                      # 速くなったか？判定結果が出る
rust-anything gate rust/fuzzy-rs                       # これに通れば完了
```

rust-anything 自体はコードを自動翻訳しません。Rust コードを書くのはあなたのコーディングエージェント（Claude Code、Codex、Cursor、OpenCode など）またはあなた自身です。rust-anything が提供するのは、エージェントが推測では確実に当てられない部分です: どこに時間がかかっているか、何をどの順番で移植すべきか、既存のコードが API を変えずに Rust を呼び出すためのブリッジ、移植版が一致させるべき正確な挙動、安全に出荷する手段、そして移植の完了を判定するゲートです。

出力が同じだけでは不十分です。アプリが外部に送るものも記録すれば、メールを 2 回送ってしまう書き直しは失敗として検出されます:

<p align="center">
  <img src="assets/effects.gif" alt="--effects smtp 付きの app record がサインアップごとに 1 通のウェルカムメールを記録。同じ出力を表示しつつメールを 2 回送る書き直しに対して app replay は 0/2 件のケースが合格、effects で失敗と報告し、レポートには余分なメールが表示されます。" width="820">
  <br>
  <sub><code>app record --effects</code> と <code>app replay</code> の実際の実行例。<a href="whole-apps.md#side-effects-email-webhooks-and-databases">副作用</a>を参照。</sub>
</p>

## 対応言語

| 言語 | 移植単位 | ブリッジ | 記録とリプレイの実行方法 |
| --- | --- | --- | --- |
| TypeScript/JavaScript | フォルダまたはファイル | napi-rs または wasm-bindgen（自動生成されたバイナリワイヤプロトコル使用、クラスと async にも対応） | Node で `.ts` ソースを直接実行 |
| Python | パッケージまたはモジュール | PyO3（abi3）、直接型変換、型付き例外 | Python で元のモジュールをインポート |
| C | `.c` ファイル | 元の関数名を持つ `extern "C"` 関数とヘッダ | 元のコードを共有ライブラリにコンパイルし、ctypes 経由でケースごとに fork して呼び出し |
| Go | パッケージ | 型付き cgo 呼び出しと自動生成バイナリワイヤ、元の API を持つパッケージとして提供 | 独立したモジュールとしてビルドされる自動生成 Go ドライバ |
| Ruby | モジュール関数を含むファイルまたはフォルダ | 元のモジュール配下に配置される magnus 拡張 | 自動生成 Ruby ドライバ |
| Java | `public static` メソッドのパッケージ | 型付き JNI と自動生成バイナリワイヤ、元のメソッドを持つラッパークラスとして提供 | 自動生成リフレクションドライバ |
| PHP | 関数および `public static` メソッドのフォルダ | 近日対応予定: C ABI 越しの PHP FFI | 現時点でアプリ全体に対応: CLI およびサーバー（`php -S`）向けの `app record`、`app replay`、フロントドア |
| C# | `public static` メソッドのフォルダ | 近日対応予定: C ABI 越しの P/Invoke | 現時点でアプリ全体に対応: CLI および ASP.NET サーバー（`dotnet <app>.dll`）向けの `app record`、`app replay`、フロントドア |

C 言語を除くすべての言語でバッチクラス（`--batch`）も生成されます。要素は一度だけ境界を越えて Rust 側に保持され、呼び出しのたびに全コアを使って全要素を一括処理します。

言語はリポジトリから自動検出されます（`go.mod`、`pom.xml`、`.csproj` または `.sln`、`pyproject.toml`、`Gemfile`、`composer.json`、`package.json`、該当しない場合は最も多いソースファイルから判断）。`--lang` で上書きもできます。型情報はソースから取得します: TS の型、Python の型ヒント、C/Go/Java/C# のシグネチャ、Ruby の YARD タグ、PHP の型および phpdoc。型注釈がなく数値として使われている引数は数値型と推論されます。

PHP については、`analyze` で PHP アプリの移行計画を立てられ（Composer パッケージと PSR-4 オートローディング、緩い比較や整数オーバーフローによる float 化などのリスク、Composer パッケージからクレートへのマッピング）、PHP の CLI やサーバーを記録・リプレイして Rust 製フロントドアの背後に配置できます。PHP の FFI を通じたユニット単位の Rust 移植は次のステップです（[issue #2](https://github.com/erkamyaman/rust-anything/issues/2)）。

C# も同じように動作します: `analyze` は `.csproj` プロジェクトとその NuGet パッケージ（クレートにマッピング）を読み取り、`using` ディレクティブをフォルダに解決して、C# 特有の挙動の違い（`decimal`、`dynamic`、LINQ の遅延クエリ、`Task`、`lock`、リフレクション）を指摘します。C# の CLI や ASP.NET サーバーは記録・リプレイ・Rust フロントドア化が可能で、P/Invoke を用いたユニット単位の移植が次のステップです。

## 詳しく知る

- [仕組み](how-it-works.md): 移植ループ、`bridge` と `analyze` が判断すること、ホットパス、検出できないもの、実際の移植に必要なこと、元の実装への追従（`map`）
- [アプリ全体の移植](whole-apps.md): サーバー、CLI、ライブラリ。フロントドアから `app lift` まで
- [例とケーススタディ](examples.md): x18n と、6 つの言語の実コード
- [実プロジェクトの移植](real-world.md): json-server、Fuse.js、vercel/ms とベンチマーク
- [インストール、更新、開発](install.md): すべてのインストール方法、Windows、ソースからのビルド
- [よくある質問](faq.md) · [ステータス](STATUS.md) · [MCP ツール](tools.md)

## スポンサー

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> は無料のオープンソースです。お役に立ちましたら、ぜひスポンサーをご検討ください。<br>
  皆様のご支援が開発を支えます。
</p>

<p align="center">
  <a href="https://coderabbit.link/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="企業スポンサー: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="スポンサー: Santosh Yadav、Hakan Soytürk、Ali Yilmaz、Ashley Hunter、Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/%E3%82%B9%E3%83%9D%E3%83%B3%E3%82%B5%E3%83%BC%E3%81%AB%E3%81%AA%E3%82%8B-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="スポンサーになる"></a>
  <a href="https://buymeacoffee.com/erkamyaman"><img src="https://img.shields.io/badge/Buy_me_a_coffee-%E2%98%95-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee"></a>
</p>

## 貢献

Issue や Pull Request を歓迎します。[CONTRIBUTING.md](../CONTRIBUTING.md) にセットアップ手順、テストの配置場所、新しい言語の追加方法が記載されています。設計上の選択とその理由は [DECISIONS.md](../DECISIONS.md) にあります。実際の移植事例（ケーススタディ）も特に歓迎しています。セキュリティ上の問題は非公開でご報告いただき（[SECURITY.md](../SECURITY.md)）、[行動規範](../CODE_OF_CONDUCT.md)をご確認ください。

## ライセンス

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman)。あなたのリポジトリ内の移植版および生成コードはすべてあなたのものです。
