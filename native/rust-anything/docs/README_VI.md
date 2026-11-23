<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="The rust-anything crab">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · Tiếng Việt · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a></sub></p>
<p align="center"><sub>Bản dịch này do cộng đồng duy trì và có thể chậm hơn <a href="../README.md">README tiếng Anh</a>.</sub></p>

<p align="center"><b>Viết lại ứng dụng bằng Rust, và chứng minh nó vẫn hoạt động y như cũ.</b><br>
Công cụ không tự dịch code: agent lập trình của bạn (hoặc chính bạn) viết Rust, còn rust-anything kiểm tra nó với bản gốc.</p>

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

<p align="center"><b><a href="#quick-start">Bắt đầu nhanh</a></b> · <a href="https://github.com/erkamyaman/rust-anything/releases">Releases</a> · <a href="#quick-start">Claude Code plugin</a> · <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=rust-anything">MCP Registry</a> · <a href="real-world.md">Các bản port thực tế</a> · <a href="faq.md#faq">FAQ</a><br>
<sub><a href="#languages">Ngôn ngữ</a> · <a href="how-it-works.md#how-it-works">Cách hoạt động</a> · <a href="whole-apps.md#port-a-whole-app">Chuyển cả ứng dụng</a> · <a href="../LICENSE">MIT</a></sub><br>
Làm tại Thổ Nhĩ Kỳ 🇹🇷 bởi <a href="https://github.com/erkamyaman">erKam Yaman</a> · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ Tài trợ cho tôi</a> · Được tài trợ bởi <a href="https://coderabbit.link/erkamyaman"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center">Mới trong 2.0: <a href="real-world.md#prettier">Prettier đã được port sang Rust</a>. Nhanh hơn 19x · 15,861 trên 17,224 lần chạy đã ghi chạy hoàn toàn bằng Rust · tất cả giống từng byte</p>

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier và bản port Rust định dạng cùng 59 file thực tế, mỗi file một tiến trình: Prettier mất 3.63 s, bản port Rust mất 0.20 s, đầu ra giống nhau đến từng byte." width="820">
  <br>
  <sub>Prettier so với bản port Rust trên 59 file thực tế lấy từ ms, Fuse.js và json-server (Apple M4). Đầu ra giống nhau đến từng byte. <a href="real-world.md#prettier">Cách đo</a>.</sub>
</p>

<p align="center"><sub>PHP và C#: hiện hỗ trợ ghi, phát lại và front door; port từng đơn vị sẽ có tiếp theo.</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>Ghi lại (Record)</b><br>
      <code>record</code> chạy chương trình gốc với các ca biên do công cụ tạo ra, các semantic trap và input của bạn. Công cụ lưu lại giá trị trả về hoặc lỗi của từng ca.
    </td>
    <td width="25%" valign="top">
      <b>Cầu nối (Port)</b><br>
      <code>bridge</code> tạo crate Rust và lớp bọc để các phần khác tiếp tục gọi API cũ. Phần Rust do bạn hoặc coding agent viết.
    </td>
    <td width="25%" valign="top">
      <b>Kiểm tra (Check)</b><br>
      <code>diff</code> chạy lại toàn bộ ca đã ghi trên bản Rust. Chỉ phát hành khi mọi kết quả đều khớp và <code>gate</code> đạt.
    </td>
    <td width="25%" valign="top">
      <b>Nâng cấp (Lift)</b><br>
      Khi replay khớp hết mà không cần fallback, và mọi unit đều qua gate, <code>app lift</code> chuyển ứng dụng thành app Rust thuần. Code gốc vẫn nguyên vẹn.
    </td>
  </tr>
</table>

**Viết lại toàn bộ ứng dụng bằng Rust, từng phần một.** rust-anything ghi lại cách ứng dụng Node.js (TypeScript/JavaScript), Python, C, Go, Ruby hoặc Java hoạt động. Sau đó, công cụ đối chiếu từng phần Rust với bản ghi trước khi chuyển sang phần tiếp theo, cho đến khi ứng dụng chạy hoàn toàn bằng Rust. Dùng được cho CLI, HTTP server và thư viện. Rust do coding agent hoặc bạn viết. Công cụ chỉ kiểm tra được những gì có trong bản ghi; xem thêm [Những gì công cụ không kiểm tra được](how-it-works.md#what-it-cant-catch). Có thể chuyển cả công cụ và server, hoặc chỉ phần chạy chậm, từng hot path một:

| Dự án thực tế, đầu ra giống bản gốc | Bản gốc | Với rust-anything |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2, 2,360 file JS/TS từ dòng lệnh | 128.6 s | **6.9 s: nhanh hơn 19x** |
| Prettier, 6,305 file Markdown từ dòng lệnh | 352 s | **22.5 s: nhanh hơn 15.6x** |
| [json-server](real-world.md#json-server), cả một Node server: bộ nhớ khi rảnh | 74 MB | **3.5 MB**, một binary 5.3 MB |
| json-server, từ lúc khởi động đến phản hồi đầu tiên | 79 đến 85 ms | **8 đến 10 ms** |
| Tìm kiếm [Fuse.js](real-world.md#fusejs) trên 50,000 chuỗi, mỗi truy vấn | 91.2 ms | **2.8 ms: nhanh hơn 32x** |

Prettier: 15,861 trong 17,224 lần chạy đã ghi được định dạng hoàn toàn bằng Rust trên 10 ngôn ngữ (JavaScript, TypeScript, JSX, JSON, GraphQL, YAML, Markdown, CSS, SCSS, Less), và cả 17,224 lần đều khớp với Prettier đến từng byte, vì phần nào chưa port sẽ fallback về Prettier. json-server: 514/514 ca HTTP đã ghi khớp trên một binary Rust thuần. Đo trên Apple M4, mỗi phép đo chạy hai lần, mọi đầu ra đều được so sánh trước khi đo thời gian ([chi tiết](real-world.md)). Gọi Rust riêng cho từng hàm nhỏ thường còn *chậm hơn* JIT; rust-anything tìm vòng lặp đáng chuyển rồi đưa vòng lặp đó sang Rust.

## Chỉ phần chạy chậm

Bạn không cần port cả ứng dụng. Hãy trỏ `go` vào một lệnh chạy quá chậm, công cụ sẽ port đúng một hàm đang tốn thời gian:

```sh
rust-anything go --agent claude -- python3 app.py
```

Công cụ profile lần chạy, chọn hàm ngoài cùng đáng chuyển (vòng lặp, không phải lời gọi nhỏ bên trong nó, và gom thành batch nếu profile cho thấy nên làm vậy), ghi lại hàm đó trên input tự tạo cùng các lời gọi mà lần chạy của bạn thực hiện, rồi giao `src/port.rs` cho agent của bạn (`claude`, `codex` hoặc bất kỳ lệnh nào đọc prompt từ stdin; nếu không có `--agent`, công cụ viết `TASK.md` cho bạn rồi dừng). Công cụ chỉ bật hàm đó khi mọi ca đã ghi đều khớp và `gate` đạt. Sau đó:

```sh
rust-anything report rust/dedupe-rs     # REPORT.md và card.svg cho pull request
rust-anything disable rust/dedupe-rs    # trở về bản gốc, giống đến từng byte
```

Hàng [Fuse.js](real-world.md#fusejs) trong bảng trên chính là dạng này: hàm tính điểm của Fuse cho một chuỗi, gom thành batch trên cả danh sách, được chuyển sang Rust, còn khóa của object, tìm kiếm mở rộng và phần còn lại vẫn ở JavaScript, nhờ đó mỗi truy vấn trên 50,000 chuỗi giảm từ 91.2 ms xuống 2.8 ms. Hãy thử `go` trên [`tests/fixtures/go-fuzzy`](../tests/fixtures/go-fuzzy), một chương trình dedupe Python 20 dòng. Mọi cách bật và tắt một bản port: [Switch a port on and off](how-it-works.md#switch-a-port-on-and-off).

<p align="center">
  <img src="assets/json-server.gif" alt="json-server dưới dạng Node server và dưới dạng một binary Rust: bộ nhớ khi rảnh 74 MB và 3.5 MB, từ khởi động đến phản hồi đầu tiên 79 đến 85 ms và 8 đến 10 ms, 514 trên 514 ca HTTP đã ghi đều khớp." width="820">
  <br>
  <sub>json-server, cả một Node server, được lift thành binary Rust thuần (Apple M4). <a href="real-world.md#json-server">Cách đo</a>.</sub>
</p>

> rust-anything là dự án độc lập, không liên kết với Rust Project hay Rust Foundation, cũng không được hai tổ chức này bảo trợ. "Rust" là nhãn hiệu của Rust Foundation.

> [!WARNING]
> `record` và `app record` chạy code gốc trên nhiều input tự tạo, gọi từng hàm hoặc cả ứng dụng. `app record` chạy mỗi ca hai lần. `diff`, `gate` và `app replay` chạy lại các ca đó trên bản port hoặc bản gốc. Nếu code ghi file, truy cập DB hay gọi mạng, những thao tác đó sẽ chạy lại theo từng ca. Hãy chạy trên bản sao của project, dùng test DB và đừng để thông tin xác thực thật trong biến môi trường.

<h2 id="quick-start">Bắt đầu nhanh</h2>

Dán đoạn này vào Claude Code, Codex, Cursor, Gemini CLI hoặc OpenCode để agent tự cài:

```text
Install rust-anything: run `cargo install rust-anything`, then `rust-anything setup`.
Tell me when to restart you.
```

Hoặc tự tay cài:

```sh
cargo install rust-anything   # from crates.io
rust-anything setup           # registers it with Claude Code, Codex, Cursor, Gemini CLI và OpenCode
```

Chưa có bộ công cụ Rust? Tải binary dựng sẵn cho macOS, Linux hoặc Windows ở mục [GitHub release](https://github.com/erkamyaman/rust-anything/releases). Với project JavaScript, chạy `npm install -g rust-anything` để cài binary đó.

Trong Claude Code, bạn cũng có thể cài dưới dạng plugin. Plugin gồm MCP server và skill; cần thêm CLI ở trên vào `PATH` trước:

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

`setup` thêm MCP server cho từng agent tìm thấy, rồi cài workflow skill vào đúng thư mục agent đọc skill. Lệnh này giữ nguyên các cấu hình khác. Có thể chạy lại; dùng `--remove` để gỡ.

Sau đó, bạn có thể yêu cầu agent:

- *"Dùng rust-anything viết lại CLI này bằng Rust từ đầu đến cuối, rồi chứng minh nó chạy như bản cũ."*
- *"Tìm nguyên nhân chỗ search chạy chậm bằng rust-anything, rồi chuyển vòng lặp nóng sang Rust."*
- *"Lập kế hoạch chuyển `src/parser/` sang Rust bằng rust-anything. Bắt đầu với unit đầu tiên và kiểm tra kết quả."*
- *"Benchmark bản port Rust trong `rust/text-rs` với bản gốc rồi cho tôi biết có nên phát hành không."*

<h3 id="works-with">Tương thích</h3>

| Agent | Thành phần `rust-anything setup` cài |
| --- | --- |
| Claude Code | MCP server in `~/.claude.json`, skill in `~/.claude/skills/` |
| Codex | MCP server in `~/.codex/config.toml`, skill in `~/.codex/skills/` |
| Cursor | MCP server in `~/.cursor/mcp.json` |
| Gemini CLI | MCP server in `~/.gemini/settings.json` |
| OpenCode | MCP server in `~/.config/opencode/opencode.json` (`mcp.servers`), skill in `~/.config/opencode/skills/` |
| các MCP client khác | tự thêm `rust-anything mcp` làm stdio server (xem [Install](install.md#install)) |

Trước khi sửa file có sẵn, `setup` lưu một bản sao cạnh file đó (`<file>.rust-anything.bak`).

Để tự cấu hình OpenCode, hãy đặt server vào `~/.config/opencode/opencode.json` (`command` là toàn bộ argv) và chép [`skills/rust-anything/`](../skills/rust-anything) vào `~/.config/opencode/skills/rust-anything/`:

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

Hoặc tự chạy từng bước:

```sh
rust-anything profile . -- node bench.mjs              # what is hot, and who calls it
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# write src/port.rs (or let your agent), then:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # every recorded case must match
rust-anything bench rust/fuzzy-rs                      # faster? the verdict says
rust-anything gate rust/fuzzy-rs                       # done means this passes
```

rust-anything không tự dịch code: bạn hoặc coding agent (Claude Code, Codex, Cursor, OpenCode, ...) sẽ viết phần Rust. rust-anything cung cấp những gì agent khó đoán chính xác: thời gian tiêu tốn ở đâu, nên chuyển phần nào trước và theo thứ tự nào, một cầu nối để phần code còn lại gọi Rust mà không phải đổi API, hành vi chính xác mà bản port phải khớp, một cách phát hành an toàn và một gate cho biết khi nào bản port hoàn tất.

Đầu ra giống nhau là chưa đủ. Hãy ghi lại cả những gì ứng dụng gửi đi, và một bản viết lại gửi mail hai lần sẽ bị đánh trượt:

<p align="center">
  <img src="assets/effects.gif" alt="app record với --effects smtp ghi lại một email chào mừng cho mỗi lượt đăng ký; app replay trên một bản viết lại in ra cùng đầu ra nhưng gửi mail hai lần báo 0/2 ca đạt, trượt ở phần effects, và báo cáo hiển thị email bị gửi thừa." width="820">
  <br>
  <sub>Một lần chạy thật của <code>app record --effects</code> và <code>app replay</code>. Xem <a href="whole-apps.md#side-effects-email-webhooks-and-databases">Side effects</a>.</sub>
</p>

<h2 id="languages">Ngôn ngữ</h2>

| Ngôn ngữ | Đơn vị | Cầu nối | Ghi và chạy lại bằng |
| --- | --- | --- | --- |
| TypeScript/JavaScript | thư mục hoặc tệp | napi-rs hoặc wasm-bindgen, dùng wire nhị phân do công cụ tạo; hỗ trợ class và async | Node, chạy trực tiếp mã nguồn `.ts` |
| Python | package hoặc module | PyO3 (abi3), chuyển đổi trực tiếp, giữ nguyên kiểu exception | Python, import trực tiếp code gốc |
| C | tệp `.c` | hàm `extern "C"` giữ nguyên tên gốc, kèm header | code gốc được biên dịch thành shared library, gọi qua ctypes; mỗi ca chạy trong một fork |
| Go | package | lời gọi cgo có kiểu rõ ràng, dùng wire nhị phân do công cụ tạo; nằm trong package giữ nguyên API | Go driver do công cụ tạo, build thành module riêng |
| Ruby | tệp hoặc thư mục chứa các module có function | extension magnus nằm trong các module gốc | Ruby driver do công cụ tạo |
| Java | package chứa các hàm `public static` | JNI có kiểu rõ ràng, dùng wire nhị phân do công cụ tạo; wrapper class giữ nguyên method gốc | reflection driver do công cụ tạo |
| PHP | thư mục chứa function và method `public static` | sắp tới: PHP FFI dựa trên C ABI | có thể chạy với cả ứng dụng ngay hôm nay: `app record`, `app replay` và front door cho CLI lẫn server (`php -S`) |
| C# | thư mục chứa các method `public static` | sắp tới: P/Invoke dựa trên C ABI | có thể chạy với cả ứng dụng ngay hôm nay: `app record`, `app replay` và front door cho CLI lẫn server ASP.NET (`dotnet <app>.dll`) |

Mọi ngôn ngữ trừ C đều có thêm batch class (`--batch`): dữ liệu chỉ qua cầu nối một lần rồi nằm trong Rust, mỗi lần gọi sẽ xử lý cả lô trên mọi core.

Công cụ nhận diện ngôn ngữ qua repo dựa trên các file như `go.mod`, `pom.xml`, `.csproj`, `.sln`, `pyproject.toml`, `Gemfile`, `composer.json`, `package.json` hoặc phần mở rộng xuất hiện nhiều nhất. Muốn chỉ định ngôn ngữ, dùng `--lang`. Kiểu dữ liệu được đọc từ source: type của TypeScript, type hint của Python, chữ ký hàm của C, Go, Java và C#, YARD tag của Ruby, type và phpdoc của PHP. Tham số không khai báo kiểu nhưng được dùng như số sẽ được suy luận là kiểu số.

Với PHP, `analyze` lập kế hoạch chuyển app PHP, gồm package Composer và PSR-4 autoloading, các rủi ro như so sánh lỏng lẻo hay tràn số nguyên thành float, và ánh xạ package Composer sang crate. Hiện có thể ghi lại, replay và đặt Rust front door trước CLI hoặc server PHP. Bước tiếp theo là chuyển từng unit sang Rust qua PHP FFI ([issue #2](https://github.com/erkamyaman/rust-anything/issues/2)).

C# cũng hoạt động theo cách tương tự. `analyze` đọc project `.csproj` và package NuGet, ánh xạ package sang crate, phân giải `using` thành thư mục rồi cảnh báo về các điểm riêng của C# như `decimal`, `dynamic`, deferred query của LINQ, `Task`, `lock` và reflection. Hiện có thể ghi lại, replay và đặt Rust front door trước CLI C# hoặc server ASP.NET. Tiếp theo là chuyển các unit qua P/Invoke.

<h2 id="learn-more">Đọc thêm</h2>

- [Cách hoạt động](how-it-works.md): quy trình, cách `bridge` và `analyze` đưa ra quyết định, hot path, những gì công cụ không kiểm tra được, một bản port thực sự cần những gì, và cách theo kịp bản gốc (`map`)
- [Chuyển cả ứng dụng](whole-apps.md): chuyển server, CLI và thư viện từ front door đến `app lift`
- [Ví dụ và tình huống thực tế](examples.md): x18n và code thực tế bằng sáu ngôn ngữ
- [Các bản port thực tế](real-world.md): json-server, Fuse.js, vercel/ms và benchmark
- [Cài đặt, cập nhật và phát triển](install.md): các cách cài, hướng dẫn cho Windows và cách build từ source
- [Câu hỏi thường gặp](faq.md) · [Trạng thái dự án](STATUS.md) · [Công cụ MCP](tools.md)

## Nhà tài trợ

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> là dự án mã nguồn mở miễn phí. Nếu dự án hữu ích với bạn, hãy cân nhắc tài trợ.<br>
  Khoản tài trợ giúp dự án tiếp tục được phát triển.
</p>

<p align="center">
  <a href="https://coderabbit.link/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="Nhà tài trợ doanh nghiệp: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="Nhà tài trợ: Santosh Yadav, Hakan Soytürk, Ali Yilmaz, Ashley Hunter, Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/Become_a_sponsor-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="Trở thành nhà tài trợ"></a>
  <a href="https://buymeacoffee.com/erkamyaman"><img src="https://img.shields.io/badge/Buy_me_a_coffee-%E2%98%95-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee"></a>
</p>

## Đóng góp

Hoan nghênh issue và pull request. [CONTRIBUTING.md](../CONTRIBUTING.md) hướng dẫn cài đặt môi trường, vị trí đặt test và cách thêm ngôn ngữ. Các lựa chọn thiết kế cùng lý do được ghi trong [DECISIONS.md](../DECISIONS.md). Bạn cũng có thể chia sẻ những lần port ứng dụng thực tế. Nếu phát hiện vấn đề bảo mật, hãy báo riêng theo hướng dẫn trong [SECURITY.md](../SECURITY.md), đồng thời đọc [quy tắc ứng xử](../CODE_OF_CONDUCT.md).

## Giấy phép

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). Các bản port và code được tạo trong repo thuộc về bạn.