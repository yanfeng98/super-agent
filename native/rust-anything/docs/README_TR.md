<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="rust-anything yengeci">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><sub><a href="../README.md">English</a> · Türkçe · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a></sub></p>
<p align="center"><sub>Bu çeviri topluluk tarafından sürdürülmektedir ve <a href="../README.md">İngilizce README</a>'nin gerisinde kalabilir.</sub></p>

<p align="center"><b>Uygulamanızı Rust ile yeniden yazın ve hâlâ aynı davrandığını kanıtlayın.</b><br>
Kodu kendisi çevirmez: Rust'ı kodlama ajanınız (ya da siz) yazar, rust-anything onu orijinaline karşı doğrular.</p>

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

<p align="center"><b><a href="#hızlı-başlangıç">Hızlı başlangıç</a></b> · <a href="https://github.com/erkamyaman/rust-anything/releases">Releases</a> · <a href="#hızlı-başlangıç">Claude Code plugin</a> · <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=rust-anything">MCP Registry</a> · <a href="real-world.md">Gerçek dünya portları</a> · <a href="faq.md#faq">SSS</a><br>
<sub><a href="#diller">Diller</a> · <a href="how-it-works.md#how-it-works">Nasıl çalışır</a> · <a href="whole-apps.md#port-a-whole-app">Tüm uygulamalar</a> · <a href="../LICENSE">MIT</a></sub><br>
Türkiye'de 🇹🇷 <a href="https://github.com/erkamyaman">erKam Yaman</a> tarafından yapıldı · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ Sponsor olun</a> · Sponsor: <a href="https://coderabbit.link/erkamyaman"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center">2.0'da yeni: <a href="real-world.md#prettier">Prettier, Rust'a port edildi</a>. 19x daha hızlı · 17,224 kayıtlı çalıştırmanın 15,861'si tamamen Rust'ta · hepsi bayt bayt aynı</p>

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier ve Rust portu aynı 59 gerçek dosyayı, dosya başına bir süreçle biçimlendiriyor: Prettier 3.63 s, Rust portu 0.20 s sürüyor ve çıktı bayt bayt aynı." width="820">
  <br>
  <sub>ms, Fuse.js ve json-server'dan alınan 59 gerçek dosyada Prettier ve Rust portu (Apple M4). Bayt bayt aynı çıktı. <a href="real-world.md#prettier">Nasıl ölçüldü</a>.</sub>
</p>

<p align="center"><sub>PHP ve C#: bugün kayıt, tekrar oynatma ve ön kapı; birim portları sırada.</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>Kaydet</b><br>
      <code>record</code>, orijinali üretilmiş uç durumlar, anlamsal tuzaklar ve sizin girdilerinizle çalıştırır; döndürdüğünü ya da fırlattığını kaydeder.
    </td>
    <td width="25%" valign="top">
      <b>Port et</b><br>
      <code>bridge</code>, Rust crate'ini ve bir wrapper üretir; böylece çağıranlar orijinal API'yi kullanmaya devam eder. Rust'ı siz ya da ajanınız yazar.
    </td>
    <td width="25%" valign="top">
      <b>Kontrol et</b><br>
      <code>diff</code>, kaydedilen her vakayı Rust'a karşı yeniden oynatır; hepsi eşleşip <code>gate</code> geçene kadar hiçbir şey yayınlanmaz.
    </td>
    <td width="25%" valign="top">
      <b>Lift</b><br>
      Yeniden oynatma fallback olmadan geçtiğinde ve her birim gate'i geçtiğinde, <code>app lift</code> onu saf bir Rust uygulamasına dönüştürür. Orijinale asla dokunulmaz.
    </td>
  </tr>
</table>

**Uygulamanızın tamamını Rust ile, parça parça yeniden yazın.** rust-anything, Node.js
(TypeScript/JavaScript), Python, C, Go, Ruby veya Java uygulamanızın bugün ne yaptığını kaydeder,
ardından Rust ile yeniden yazılan her parçayı bir sonrakine geçmeden önce bu kayda karşı kontrol
eder; ta ki uygulama tamamen Rust olana kadar. CLI'lar, HTTP sunucuları ve kütüphaneler; Rust'ı
kodlama ajanınız (ya da siz) yazar. Yalnızca kaydın kapsadığını yakalayabilir: bkz.
[Neyi yakalayamaz](how-it-works.md#what-it-cant-catch). Araçların ve sunucuların tamamı ya da
yalnızca yavaş kısım, her seferinde bir hot path:

| Gerçek proje, orijinaliyle aynı çıktı | Orijinal | rust-anything ile |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2, komut satırından 2,360 JS/TS dosyası | 128.6 s | **6.9 s: 19x daha hızlı** |
| Prettier, komut satırından 6,305 Markdown dosyası | 352 s | **22.5 s: 15.6x daha hızlı** |
| [json-server](real-world.md#json-server), bir Node sunucusunun tamamı: boştayken bellek | 74 MB | **3.5 MB**, tek bir 5.3 MB binary |
| json-server, başlatmadan ilk yanıta | 79 ila 85 ms | **8 ila 10 ms** |
| 50,000 string üzerinde [Fuse.js](real-world.md#fusejs) araması, sorgu başına | 91.2 ms | **2.8 ms: 32x daha hızlı** |

Prettier: 17,224 kayıtlı çalıştırmanın 15,861'si 10 dilde (JavaScript, TypeScript, JSX, JSON,
GraphQL, YAML, Markdown, CSS, SCSS, Less) tamamen Rust'ta biçimlendirilir ve 17,224'ünün tamamı
Prettier ile bayt bayt eşleşir; henüz port edilmemiş her şey Prettier'a geri düşer. json-server:
kaydedilen 514/514 HTTP vakası saf bir Rust binary'si üzerinde eşleşir. Apple M4 üzerinde, her biri
iki kez çalıştırılarak ölçülmüştür; her çıktı, süre ölçülmeden önce karşılaştırılır
([ayrıntılar](real-world.md)). Her küçük fonksiyon için Rust'ı ayrı ayrı çağırmak genellikle bir
JIT'ten *daha yavaştır*; rust-anything taşımaya değer döngüyü bulur ve onu taşır.

## Sadece yavaş kısım

Uygulamanın tamamını port etmeniz gerekmez. `go`'yu fazla yavaş çalışan bir komuta yöneltin,
zamanı yiyen tek fonksiyonu port etsin:

```sh
rust-anything go --agent claude -- python3 app.py
```

Çalıştırmayı profiller, taşımaya değer en dıştaki fonksiyonu seçer (içindeki küçük çağrıyı değil
döngüyü; profil öyle diyorsa batch olarak), onu üretilen girdiler ve çalıştırmanızın yaptığı
çağrılarla kaydeder ve `src/port.rs` dosyasını ajanınıza verir (`claude`, `codex` ya da istemi
stdin'den okuyan herhangi bir komut; `--agent` olmadan sizin için `TASK.md` yazar ve durur).
Fonksiyonu ancak kaydedilen her vaka eşleştiğinde ve `gate` geçtiğinde devreye alır. Sonra:

```sh
rust-anything report rust/dedupe-rs     # pull request için REPORT.md ve card.svg
rust-anything disable rust/dedupe-rs    # bayt bayt orijinale geri dönüş
```

Yukarıdaki [Fuse.js](real-world.md#fusejs) satırı da bu biçimdedir: Fuse'un tek bir string için
skoru, listenin tamamı üzerinde batch olarak Rust'a taşındı; nesne anahtarları, genişletilmiş
arama ve geri kalan her şey JavaScript'te kaldı. 50,000 string üzerinde sorgu başına 91.2 ms'den
2.8 ms'ye. `go`'yu 20 satırlık bir Python dedupe olan
[`tests/fixtures/go-fuzzy`](../tests/fixtures/go-fuzzy) üzerinde deneyin. Bir portu açıp
kapatmanın bütün yolları: [Switch a port on and off](how-it-works.md#switch-a-port-on-and-off).

<p align="center">
  <img src="assets/json-server.gif" alt="Node sunucusu olarak ve tek bir Rust binary'si olarak json-server: boştayken bellek 74 MB ve 3.5 MB, başlatmadan ilk yanıta 79 ila 85 ms ve 8 ila 10 ms, kaydedilen 514 HTTP vakasının 514'ü eşleşiyor." width="820">
  <br>
  <sub>Bir Node sunucusunun tamamı olan json-server, saf bir Rust binary'sine dönüştürüldü (Apple M4). <a href="real-world.md#json-server">Nasıl ölçüldü</a>.</sub>
</p>

> rust-anything bağımsız bir projedir; Rust Project veya Rust Foundation ile bağlantılı değildir ve
> onlar tarafından desteklenmez. "Rust", Rust Foundation'ın ticari markasıdır.

> [!WARNING]
> `record` ve `app record` orijinal kodunuzu çalıştırır; her fonksiyonu (ya da uygulamanın
> tamamını) çok sayıda üretilmiş girdiyle çağırır ve `app record` her vakayı iki kez çalıştırır.
> `diff`, `gate` ve `app replay` aynı vakaları port ya da orijinal üzerinde yeniden çalıştırır.
> Dosya yazan, bir veritabanıyla konuşan veya ağa çağrı yapan kod bunu her vaka için bir kez yapar.
> Bunları projenin bir kopyası üzerinde, bir test veritabanıyla ve ortamda hiçbir production
> kimlik bilgisi olmadan çalıştırın.

## Hızlı başlangıç

Bunu Claude Code, Codex, Cursor, Gemini CLI veya OpenCode'a yapıştırın ve kurulumu ajanınıza bırakın:

```text
rust-anything'i kur: `cargo install rust-anything`, ardından `rust-anything setup` çalıştır.
Seni ne zaman yeniden başlatmam gerektiğini söyle.
```

Ya da kendiniz:

```sh
cargo install rust-anything   # crates.io'dan
rust-anything setup           # Claude Code, Codex, Cursor, Gemini CLI ve OpenCode'a kaydeder
```

Henüz Rust toolchain'iniz yok mu? macOS, Linux ve Windows için hazır derlenmiş binary'ler her
[GitHub release](https://github.com/erkamyaman/rust-anything/releases)'e eklenmiştir; JavaScript
projeleri için `npm install -g rust-anything` aynı binary'yi kurar.

Claude Code'da bunu bir plugin olarak da kurabilirsiniz (MCP sunucusu ve skill; yukarıdaki CLI
`PATH` içinde olmalıdır):

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

`setup`, bulduğu her ajana MCP sunucusunu ekler ve iş akışı skill'ini ajanın skill'leri okuduğu
yere kurar. Diğer ayarlarınızı korur, tekrar çalıştırmak güvenlidir ve `--remove` yaptıklarını geri alır.
Ardından ajanınıza örneğin şunları sorun:

- *"Bu CLI'ı rust-anything ile baştan sona Rust'ta yeniden yaz ve aynı davrandığını kanıtla."*
- *"Bu arama yavaş geliyor. rust-anything ile profilini çıkar ve sıcak döngüyü Rust'a taşı."*
- *"`src/parser/`'ın rust-anything ile Rust'a geçişini planla; ilk birimi port et ve kanıtla."*
- *"`rust/text-rs` içindeki Rust portunu orijinaline karşı benchmark'la ve yayınlamaya değer mi söyle."*

### Desteklenen ajanlar

| Ajan | `rust-anything setup` şunları ekler |
| --- | --- |
| Claude Code | `~/.claude.json` içinde MCP sunucusu, `~/.claude/skills/` içinde skill |
| Codex | `~/.codex/config.toml` içinde MCP sunucusu, `~/.codex/skills/` içinde skill |
| Cursor | `~/.cursor/mcp.json` içinde MCP sunucusu |
| Gemini CLI | `~/.gemini/settings.json` içinde MCP sunucusu |
| OpenCode | `~/.config/opencode/opencode.json` içinde MCP sunucusu (`mcp.servers`), `~/.config/opencode/skills/` içinde skill |
| Diğer herhangi bir MCP istemcisi | `rust-anything mcp`'yi elle stdio sunucusu olarak ekleyin (bkz. [Kurulum](install.md#install)) |

Var olan bir dosyayı değiştirmeden önce `setup` yanına bir kopyasını kaydeder (`<file>.rust-anything.bak`).

OpenCode'u elle bağlamak için sunucuyu `~/.config/opencode/opencode.json` içine koyun (`command`
argv'nin tamamıdır) ve [`skills/rust-anything/`](../skills/rust-anything) klasörünü
`~/.config/opencode/skills/rust-anything/` içine kopyalayın:

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

Ya da adımları kendiniz çalıştırın:

```sh
rust-anything profile . -- node bench.mjs              # sıcak olan ne, onu kim çağırıyor
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# src/port.rs'i yazın (ya da ajanınıza yazdırın), ardından:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # kaydedilen her vaka eşleşmeli
rust-anything bench rust/fuzzy-rs                      # daha mı hızlı? karar söyler
rust-anything gate rust/fuzzy-rs                       # bitti demek, bunun geçmesi demek
```

rust-anything kodu çevirmez: Rust'ı kodlama ajanınız (Claude Code, Codex, Cursor, OpenCode, ...) ya da siz
yazarsınız. rust-anything, bir ajanın güvenilir biçimde tahmin edemeyeceği şeyleri sağlar: zamanın
nereye gittiğini, önce neyin ve hangi sırayla port edileceğini, kodun geri kalanının Rust'ı API'si
değişmeden çağırabilmesi için bir köprüyü (bridge), portun eşleşmesi gereken davranışın tam halini,
onu yayına almanın güvenli bir yolunu ve bir portun ne zaman bittiğini söyleyen bir gate'i.

Aynı çıktı yetmez. Uygulamanın dışarı gönderdiklerini de kaydedin; iki kez e-posta atan bir yeniden yazım başarısız olur:

<p align="center">
  <img src="assets/effects.gif" alt="--effects smtp ile app record, her kayıt için bir hoş geldin e-postası kaydediyor; aynı çıktıyı basan ama iki kez e-posta atan bir yeniden yazımda app replay 0/2 vakanın geçtiğini, efektlerde başarısız olduğunu bildiriyor ve rapor fazladan e-postayı gösteriyor." width="820">
  <br>
  <sub><code>app record --effects</code> ve <code>app replay</code>'in gerçek bir çalıştırması. Bkz. <a href="whole-apps.md#side-effects-email-webhooks-and-databases">Yan etkiler</a>.</sub>
</p>

## Diller

| Dil | Birim | Köprü | Kaydeden ve yeniden oynatan |
| --- | --- | --- | --- |
| TypeScript/JavaScript | klasör veya dosya | üretilmiş bir binary wire ile napi-rs veya wasm-bindgen; sınıflar ve async dahil | `.ts` kaynaklarını çalıştıran Node |
| Python | paket veya modül | PyO3 (abi3), doğrudan dönüşüm, tipli exception'lar | orijinali import eden Python |
| C | `.c` dosyası | orijinal adlarla `extern "C"` fonksiyonlar ve bir header | paylaşımlı kütüphane olarak derlenmiş orijinal, ctypes üzerinden çağrılır, vaka başına bir fork |
| Go | paket | orijinal API'ye sahip bir pakette tipli cgo çağrıları ve üretilmiş bir binary wire | kendi modülü olarak derlenen, üretilmiş bir Go sürücüsü |
| Ruby | modül fonksiyonlarından oluşan dosya veya klasör | orijinal modüllerin altında bir magnus eklentisi | üretilmiş bir Ruby sürücüsü |
| Java | `public static` metotlardan oluşan paket | orijinal metotlara sahip wrapper sınıflarda tipli JNI ve üretilmiş bir binary wire | üretilmiş bir reflection sürücüsü |
| PHP | fonksiyonlar ve `public static` metotlardan oluşan klasör | sırada: C ABI üzerinden PHP FFI | bugün tüm uygulamalar: CLI'lar ve sunucular (`php -S`) için `app record`, `app replay` ve front door |
| C# | `public static` metotlardan oluşan klasör | sırada: C ABI üzerinden P/Invoke | bugün tüm uygulamalar: CLI'lar ve ASP.NET sunucuları (`dotnet <app>.dll`) için `app record`, `app replay` ve front door |

C hariç her dil batch sınıfları da alır (`--batch`): öğeler köprüyü bir kez geçer ve Rust'ta kalır,
her çağrı tüm çekirdeklerde hepsinin üzerinden çalışır.

Dil, repodan tespit edilir (`go.mod`, `pom.xml`, bir `.csproj` veya `.sln`, `pyproject.toml`,
`Gemfile`, `composer.json`, `package.json`, aksi halde en yaygın kaynak dosyalar); `--lang` bunu
geçersiz kılar. Tipler kaynaktan gelir: TS tipleri, Python type hint'leri, C, Go, Java ve C#
imzaları, Ruby YARD etiketleri, PHP tipleri ve phpdoc; sayı olarak kullanılan tipsiz parametreler
sayı olarak çıkarılır.

PHP için `analyze` bir PHP uygulamasını planlar (Composer paketleri ve PSR-4 autoloading,
gevşek karşılaştırma ve tam sayı taşmasının float'a dönüşmesi gibi riskler, crate'lere eşlenmiş
Composer paketleri) ve bir PHP CLI'ı ya da sunucusu kaydedilebilir, yeniden oynatılabilir ve bir
Rust front door'unun arkasına konabilir. Birimlerini PHP'nin FFI'ı üzerinden Rust'a port etmek bir
sonraki adımdır ([issue #2](https://github.com/erkamyaman/rust-anything/issues/2)).

C# da aynı şekilde çalışır: `analyze` `.csproj` projelerini ve NuGet paketlerini okur
(crate'lere eşlenmiş olarak), `using` yönergelerini klasörlere çözümler ve C#'ın farklı yaptığı
şeyleri işaretler (`decimal`, `dynamic`, LINQ'in ertelenmiş sorguları, `Task`, `lock`, reflection).
Bir C# CLI'ı veya ASP.NET sunucusu kaydedilebilir, yeniden oynatılabilir ve bir Rust front door'unun
arkasına konabilir; birimleri P/Invoke üzerinden port etmek sırada.

## Daha fazlası

- [Nasıl çalışır](how-it-works.md): döngü, `bridge` ve `analyze` neye karar verir, hot path'ler, neyi yakalayamaz, gerçek bir portun neler gerektirdiği ve orijinale ayak uydurmak (`map`)
- [Tüm uygulamayı port etmek](whole-apps.md): sunucular, CLI'lar ve kütüphaneler, front door'dan `app lift`'e kadar
- [Örnekler ve vaka çalışmaları](examples.md): x18n ve altı dilde gerçek kod
- [Gerçek dünya portları](real-world.md): json-server, Fuse.js, vercel/ms, benchmark'larıyla
- [Kurulum, güncelleme ve geliştirme](install.md): tüm kurulum yöntemleri, Windows ve kaynaktan derleme
- [SSS](faq.md) · [Durum](STATUS.md) · [MCP araçları](tools.md)

## Sponsorlar

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> ücretsiz ve açık kaynaklıdır. İşinize yarıyorsa lütfen sponsor olmayı düşünün.<br>
  Desteğiniz geliştirmenin sürmesini sağlar.
</p>

<p align="center">
  <a href="https://coderabbit.link/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="Şirket sponsoru: CodeRabbit">
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

## Katkıda bulunma

Issue'lar ve pull request'ler memnuniyetle karşılanır; [CONTRIBUTING.md](../CONTRIBUTING.md) kurulumu,
testlerin nereye gittiğini ve yeni bir dilin nasıl ekleneceğini anlatır. Tasarım kararları ve
gerekçeleri [DECISIONS.md](../DECISIONS.md) içindedir. Gerçek portların vaka çalışmaları özellikle
memnuniyetle karşılanır. Lütfen güvenlik sorunlarını gizli olarak bildirin ([SECURITY.md](../SECURITY.md))
ve [davranış kurallarına](../CODE_OF_CONDUCT.md) göz atın.

## Lisans

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). Reponuzdaki portlar ve üretilmiş kod sizindir.
