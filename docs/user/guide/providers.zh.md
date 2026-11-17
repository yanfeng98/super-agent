# 配置模型

[English](providers.md) | 中文

本指南假定你已按照[根 README](../../../README.md#run)启动 Web UI。模型变更会在下一次请求时生效，不需要重启服务器。

## 配置 DeepSeek

打开**设置 → 模型**。DeepSeek 卡片提供一个 API 密钥字段；输入密钥并保存。

![模型页：DeepSeek 卡片，及其下方的添加模型提供商入口](providers-models-page.zh.png)

密钥是只写的。保存后，页面只会收到脱敏描述符，永远不会收到明文密钥。密钥存储在 `$DSH_HOME/.credentials.yaml` 中，settings 只保留它的凭据引用。

保存了密钥且使用默认端点时，DeepSeek 的模型跟随 DeepSeek API 发布的列表。第一次需要模型时加载列表，使用期间约每十分钟在后台刷新一次，无法加载时退回内置模型。在**自定义设置**里编辑**模型目录**会改用你自己的列表，不再跟随发布的列表；点**恢复默认模型**会重新跟随并重新加载。使用发布的列表时，卡片上显示的是内置模型，因此新发布的模型只出现在模型选择器中。列表、兜底和门控规则见 [API key 提供方参考](../../../packages/llm/llm-deepseek-api-key/README.zh.md)。

## 添加第三方提供商

选择**添加模型提供商**。卡片默认打开在**第三方模型提供商**：选取 dsh 自带的提供商——列表显示的是提供商 id，例如 `anthropic`、`openai`、Kimi 编码套餐对应的 `kimi-coding`、GLM 编码套餐对应的 `zai-coding-cn`——输入其 API 密钥并保存。已安装目录会提供端点、协议和模型列表。

认证 **?** 说明已保存凭据的优先级和环境后备。密钥留空时，使用已保存凭据，或尝试读取列出的环境变量。绿色提示标明已检测到环境变量。黄色提示表示未检测到凭据变量；请输入 API 密钥或配置列出的变量。状态未确认表示读取失败。检测不验证凭据，也不测试连接。AWS profile 和 Google 应用默认凭据等提供商管理的凭据文件不在检查范围内。路由本身没有可保存 API 密钥的提供商（例如 `openai-codex`）不显示密钥字段；当该提供商提供登录流程时，卡片改为显示登录按钮，页面会展示流程报告的每条通知——要打开的页面、要输入的验证码——并询问流程需要回答的问题。提供商确认后，凭据写入本地凭据存储。关闭弹窗会撤回这次登录；重新加载后会接回仍在运行的那次；Host 已不再持有的尝试会报告为已结束，可重新开始。

已保存凭据在输入框中显示脱敏预览，下方显示绿色的使用已保存密钥提示。预览保留第一个连字符或下划线之前的标识，以及密钥的前五、后四个字符，但中间必须至少隐藏八个字符；否则整串和标识全部显示为点。计数使用 Unicode 码点。已保存密钥不显示眼睛按钮，不能显示、复制、剪切或局部编辑；键入或粘贴会整项替换。Backspace 或 Delete 会暂存整项删除。按 Backspace 或 Delete，再点**保存**，即可移除存储覆盖并使用环境认证。清空替换草稿也会暂存删除已保存密钥。保存前，**取消**会保留密钥。删除被拒绝时，设置保持不变。密钥已删除但设置失败时，警告会说明此删除无法撤销；可重试**保存**完成配置更新，或**关闭**并保留部分提交结果。清除共享凭据会影响使用同一引用的全部功能。没有环境凭据时，清除后提供商将没有认证。环境后备使用**留空使用环境认证**占位文字，输入留空时显示一条检测提示。替换草稿显示保存以替换密钥的提示。待删除提示会列出保存后使用的环境变量名（路由声明该变量时）。其他非空输入不显示认证状态文字。地址配置变量显示在**基础 URL**下方，不计作凭据。

可编辑的密钥草稿非空时，输入框内显示眼睛按钮。选择**显示密钥**检查当前输入，选择**隐藏密钥**重新遮住输入。修改输入后会重新遮住密钥。原生密码框不提供剪切；可选中草稿后按 Backspace 清空，或显示明文后再剪切。已保存凭据仍保持只写。

本构建不提供 Amazon Bedrock（`amazon-bedrock`）、Ant Ling（`ant-ling`，Ling 与 Ring 模型系列的第一方路由）、Azure（`azure`）、Baseten（`baseten`，托管其他厂商开源权重模型的第三方推理云）、Cerebras（`cerebras`，芯片厂商自建的推理云）、Cloudflare AI Gateway（`cloudflare-ai-gateway`）、Cloudflare Workers AI（`cloudflare-workers-ai`）、Fireworks（`fireworks`，转售其他厂商开源权重模型并托管自有 Ember 系列的第三方推理云）、Google Vertex AI（`google-vertex`，Google Cloud 面向同一批 Gemini 模型的 Vertex AI 端点）、Groq（`groq`，面向开源权重模型的芯片厂商推理云）、Hugging Face（`huggingface`，把每个请求转发给第三方推理服务商的路由）、MiniMax（`minimax`，国际站端点）、Mistral（`mistral`，Mistral AI 的第一方平台端点）、Moonshot AI（`moonshotai`，国际站端点）、Moonshot AI CN（`moonshotai-cn`，中国站端点）、NVIDIA（`nvidia`，芯片厂商自建的推理云，托管自家与第三方的开源权重模型）、OpenCode Zen（`opencode`，OpenCode 的按量付费 Console 网关）、Qwen Token Plan（`qwen-token-plan`，国际站端点）、Qwen Token Plan CN（`qwen-token-plan-cn`，中国站端点）、Qwen Token Plan Individual（`qwen-token-plan-individual`，个人版模型子集）、Radius（`radius`，转售其他厂商模型的网关）、Together（`together`，托管其他厂商开源权重模型的第三方推理云）、Vercel AI Gateway（`vercel-ai-gateway`，聚合网关）、Xiaomi（`xiaomi`，按量计费的开放平台）、Xiaomi Token Plan SGP（`xiaomi-token-plan-sgp`，新加坡）、Xiaomi Token Plan AMS（`xiaomi-token-plan-ams`，阿姆斯特丹）与 Z.AI Coding Plan（`zai`，Z.AI 的国际站编码套餐端点），选择器中也不会出现它们。存量 profile 若点名其中一条，`llm-pi-ai` 插件会加载失败：错误会指出该路由，从 `llm-pi-ai.config.providers` 删除该条目即可恢复插件的其他路由。`bedrock-converse-stream`、`google-vertex` 与 `azure-openai-responses` 不在自定义模型 API 的协议选项中，但 OpenAI 兼容或 `anthropic-messages` 端点仍可通过自定义模型 API 接入；Vercel AI Gateway 的端点 `ai-gateway.vercel.sh`（Anthropic Messages）、Ant Ling 端点 `api.ant-ling.com/v1`、Z.AI 国际站编码套餐端点 `api.z.ai/api/coding/paas/v4`、Mistral 端点 `api.mistral.ai/v1`、Hugging Face 的路由端点 `router.huggingface.co/v1`、Baseten 端点 `inference.baseten.co/v1`、Fireworks 端点 `api.fireworks.ai/inference/v1`（OpenAI 兼容对话补全）或 `api.fireworks.ai/inference`（Anthropic Messages）、Groq 端点 `api.groq.com/openai/v1`、Cerebras 端点 `api.cerebras.ai/v1`、NVIDIA 端点 `integrate.api.nvidia.com/v1`、Radius 网关端点 `radius.pi.dev/v1`、Together 端点 `api.together.ai/v1`（OpenAI 兼容对话补全）、OpenCode Console 端点 `opencode.ai/inference/v1`（付费模型可用该路由，免费档模型只允许 OpenCode 客户端使用）与 Xiaomi 开放平台端点 `api.xiaomimimo.com/v1` 同样适用，后者与 `xiaomi-token-plan-cn` 提供相同的四个 MiMo 模型，另有两个 UltraSpeed 变体；SGP 与 AMS token 套餐端点 `token-plan-sgp.xiaomimimo.com/v1` 与 `token-plan-ams.xiaomimimo.com/v1` 亦然，它们与 `xiaomi-token-plan-cn` 提供相同的四个 MiMo 模型；Moonshot 开放平台的 `api.moonshot.ai` 与 `api.moonshot.cn` 端点亦然，两者提供相同的四个 Kimi 模型；阿里云 token 套餐端点 `token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1` 与 `token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1` 亦然，两端提供相同的二十个模型，其中个人版套餐开放九个；这类路由需使用与被移除路由不同的 Provider ID，该 ID 仍会被拒绝。Vercel AI Gateway 的 301 个模型 id 中有 118 个在 `openrouter` 上原样出现，其余 id 需保留自定义路由才能继续使用；该路由的 78 个模型名中有 64 个也出现在 `openrouter` 上（其中 29 个换了厂商前缀），其余 14 个没有其他内置路由，需依赖自定义路由。Ant Ling 的三个模型——Ling 2.6 两个与推理模型 Ring 2.6 1T——没有其他内置路由，只有上面的自定义路由能接入。Cerebras 的两个模型在其他路由上仍然可用：`gpt-oss-120b` 为 `openrouter` 上的 `openai/gpt-oss-120b`，Qwen3.8 27B 为 `openrouter` 上的 `qwen/qwen3.8-27b`（`qwen` 后没有连字符）。Google Vertex AI 是 Google Cloud 自家端点，服务的是 `google` 已经提供的同一批模型：14 个条目全部以相同 id、相同上下文窗口与相同输入模态出现在 `google` 上，因此移除它拿掉的是一条托管入口而非模型覆盖；不过 `gemini-4-argon` 在 Vertex 上的输出上限是 65,536（`google` 为 32,768），`gemini-3.1-flash-lite-image` 在 Vertex 上另有 0.025 的缓存读取价，`google` 侧则为三个条目记录阶梯输入价。由于 `google-vertex` 协议不在自定义模型 API 的可选协议里，也没有自定义模型 API 可以替代。Mistral 的 40 个条目中有 29 个能在其他内置路由上取到同一模型（`openrouter` 的 `mistralai/*` 条目；GLM 的两条另有 `opencode-go` 提供），其余 11 个没有其他内置路由——Pixtral 两个、Magistral Medium、Leanstral 1.5、Devstral Medium、Devstral Small 的三个条目、Mistral Large 2.1、Mistral 7B 与 Mixtral 8x7B——因此移除它拿掉的不只是托管入口，还有这 11 个模型。Groq 的七个模型同样都出现在 `openrouter` 上（两个 Llama 的 id 带 `meta-llama/` 前缀），因此移除 Groq 拿掉的是一条托管入口而非模型覆盖；其端点 `api.groq.com/openai/v1` 支持 OpenAI 兼容的对话补全。Baseten 是托管其他厂商开源权重模型的第三方推理云，用单个 OpenAI 兼容端点提供这些模型：其 22 个模型 id 中有 10 个在其他内置路由上保持相同 id，另有 10 个以不同的 id 写法或显示名对应其他路由上的同一模型，其余 2 个没有任何其他内置路由——`deepseek-ai/DeepSeek-V4.1-Flash-Fast` 与 `zai-org/GLM-5.2-Fast`（`-Fast` 的价格高于同名标准款）——因此移除它去掉的是一个服务面外加这 2 个 id；其端点仍可作为 OpenAI 兼容自定义模型 API `inference.baseten.co/v1` 接入。Together 是托管其他厂商开源权重模型的第三方推理云，用单个 OpenAI 兼容端点提供这些模型：17 个条目全部能在已提供路由上取到——15 个对应 `openrouter` 上的同一模型，Llama 3.3 70B Instruct Turbo 与 Qwen2.5 7B Instruct Turbo 两个条目分别对应 `openrouter` 的非 Turbo 版本 `meta-llama/llama-3.3-70b-instruct` 与 `qwen/qwen-2.5-7b-instruct`——因此移除它拿掉的是一条托管入口而非模型覆盖。OpenCode Zen 的目录在其他路由上基本仍可取到：其 84 个模型 id 中有 57 个原样出现在其他路由上，另有 14 个在不同厂商前缀下对应同一模型，其余 13 个中有 8 个是 OpenCode 只允许其自带客户端使用的免费档模型，另外 5 个（`claude-sonnet-4`、`qwen3.5-plus`、`gemini-3-flash`、`gemini-3.1-pro`、`gpt-5-codex`）是只能通过上述自定义路由接入的付费模型。MiniMax 的国际站路由与其中文站提供同样的三个模型——`MiniMax-M2.7`、`MiniMax-M2.7-highspeed` 与 `MiniMax-M3`，能力与目录价格完全一致——因此移除它后该厂商仍由 `minimax-cn` 提供；国际站账号可通过 `anthropic-messages` 自定义路由 `https://api.minimax.io/anthropic` 接入这些模型。Z.AI 的国际站编码套餐（`zai`）与它的中国站同属一个产品：保留的 `zai-coding-cn` 承接了 7 个模型 id 中的 3 个——`glm-5.3`、`glm-5.3-flash` 与 `glm-5.3-highspeed`，能力与目录价格一致；`glm-5.2` 另见 `opencode-go`，`glm-4.7` 与 `glm-5-turbo` 另见 `openrouter`，只有 `glm-5.2-highspeed` 没有其他内置路由。Radius 转售其他厂商的模型：其 29 个模型 id 中有 26 个在内置路由上原样出现，其余三个——`balanced`、`cheap` 与 `precise`——是意图别名，指向的是其他路由已提供的模型，因此移除它去掉的是路由与计费层，而不是模型覆盖；它使用 `pi-messages` 协议，自定义路由不提供该协议，其端点仍可作为 OpenAI 兼容自定义模型 API `radius.pi.dev/v1` 接入。Kimi 编码套餐（`kimi-coding`）、同一厂商的订阅路由（`opencode-go`，按月订阅、解析同一 `OPENCODE_API_KEY`）、MiniMax 中文站（`minimax-cn`）、Z.AI 中国站编码套餐（`zai-coding-cn`）与小米中国区 token 套餐（`xiaomi-token-plan-cn`）仍然提供。迁移步骤见[升级指南](../../upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/guide.zh.md)。

## 添加自定义模型 API

对于中转站、公司网关、自建服务器或已安装目录中不存在的提供商，把卡片切换到**自定义模型 API**。提供小写 Provider ID、基础 URL、API 协议、凭据和至少一个模型。**API 协议**必须选网关实际使用的那一种，选择框提供四种：OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 和 Gemini API，在当前 profile 的 `cordis.patch.yml` 中分别存为 `openai-completions`、`openai-responses`、`anthropic-messages` 和 `google-generative-ai`。Gemini API 的基础 URL 包含 API 版本，例如 `https://gateway.example/v1beta`。一个提供商只使用一种协议，网关同时提供两种时需要建两个提供商。

![自定义模型 API 表单：Provider ID、显示名称、API 地址、API 协议、API 密钥](providers-custom-form.zh.png)

Provider ID 是永久的，因为请求、已保存会话、模型默认值和凭据引用都会使用它。如需重命名提供商，请添加新提供商并删除旧提供商。显示名称、基础 URL、协议、凭据和模型仍可编辑。

### 探测模型

在**模型目录**中选择**获取可用模型**，即可询问端点它提供哪些模型。请求使用表单当前的 API 地址、协议和密钥，已保存的提供商则用已存储的密钥；响应会打开一个可搜索的选择框，列表中已有的模型默认勾选，搜索后勾选要添加的模型、取消勾选要移除的模型，再点**更新列表**。端点未列出的已配置模型保持不变。保存或创建提供商之前不会存储任何内容。

探测读取的是常见网关公开的列表格式，但并非每个端点都用这些格式作答，所以它只是便利手段而非保证：探测失败或列表为空时，手动添加模型 ID 即可，效果完全一样。内置提供商一律由已安装目录作答，即使其 API 地址指向网关也是如此，要查看网关实际提供的模型，请通过自定义提供商探测。

## 选择模型

已配置的提供商会出现在模型选择器中。选择模型也会将其设为新会话的默认值。已发送过请求的会话会保留自身日志中记录的模型。

在**设置 → 模型**中编辑提供商，打开**自定义设置**，再拖动模型的三横线手柄，即可调整该提供商内的模型顺序。也可以聚焦手柄后按上下方向键。保存会保留顺序；取消会放弃修改。模型选择器会按该提供商保存的顺序列出模型。

如果已保存默认值指向已删除的提供商，输入框会显示**选择模型**，并在选择其他模型前阻止输入。

## 进阶配置

自动生成的[插件配置目录](../../config-catalog.zh.md)列出每个插件的所有受支持字段与默认值；[`dsh-llm-pi-ai`](../../config-catalog.zh.md#deepseek-aidsh-llm-pi-ai) 就是本页所配置的那个提供商段落。[`dsh-llm-pi-ai`](../../../packages/llm/llm-pi-ai/README.zh.md) 和 [`dsh-llm-deepseek`](../../../packages/llm/llm-deepseek/README.zh.md) 参考文档负责直接 `cordis.patch.yml` 配置、目录解析、推理控制、凭据与适配器错误。

::: tip 其他设置
模型页提供 API 密钥、显示名称、API 地址、API 协议，以及每个模型的 ID、显示名称、上下文窗口、最大输出 token 数和输入类型；自定义模型 API 提供商的模型还可以在这里设置推理等级。内置提供商模型的推理等级、请求兼容性开关、请求头、超时和重试策略在 `$DSH_HOME/profiles/<profile>/cordis.patch.yml` 中设置，也就是模型页写入的同一份文档。可以直接编辑它；浏览器与服务器在同一台机器时，也可以点击设置页顶部的**打开配置文件**打开它。适配器会在下一次请求时重新读取，无需重启任何东西。下面各小节介绍多数网关会用到的字段。

按常规方式通过 `dsh web` 启动 Web UI 时，`<profile>` 就是 `web`，完整路径为 `$DSH_HOME/profiles/web/cordis.patch.yml`。如果使用自定义 profile，请替换为启动时指定的名称。
:::

### 图片输入

在**设置 → 模型**中编辑提供商，打开**自定义设置**并展开该模型的**模型选项**。**输入类型**独占容量字段下方的一行。对于支持图片的模型，勾选**图片**并保存。没有继承图片能力的新自定义模型默认勾选**文本**。至少保留一种输入类型；仅图片模型需先勾选图片，再取消文本。

复选框将 pi-ai 模型的选择保存为 `input`，将直连 DeepSeek 适配器的选择保存为 `inputModalities`。也可以在 `$DSH_HOME/profiles/<profile>/cordis.patch.yml` 中编辑模型；例如，以下自定义 pi-ai 提供商声明了一个纯文本模型和一个视觉模型：

这些示例展示 profile patch 中的配置字段。Cordis 配置覆盖会替换完整条目配置；编辑已有覆盖项时，请保留其他 provider 和字段。

```yaml
- id: llm-pi-ai
  config:
    providers:
      my-gateway:
        apiKeyEnv: GATEWAY_API_KEY
        api: openai-completions
        baseURL: https://gateway.example/v1
        models:
          - id: legacy-chat
          - id: vision-preview
            input: [text, image]
```

Pi-ai 的 `input` 接受 `text` 和 `image`，且只作用于该模型。显式的非空选择优先。省略或为空的 `input` 先继承已安装目录的输入类型，再回退到路由的 `defaultInput`，后者默认为 `[text]`。复选框会显示这些继承值，仅打开模型行不会保存覆盖值。

DeepSeek 将省略的 `inputModalities` 视为纯文本，并拒绝空列表。取消图片还会移除该模型的 `imagePixelBudget` 和 `imageMaxBytes`，因为 DeepSeek 拒绝纯文本模型上的图片限制。以后重新启用图片且需要自定义限制时，需再次设置这些限制。

修改复选框后如需恢复继承，可在 `cordis.patch.yml` 中移除模型的 `input` 或 `inputModalities` 字段。**恢复默认模型**会移除整个模型目录覆盖，包括其他模型编辑，因此仅在需要恢复整个目录时使用。

如果你手动录入的模型全都接受图片，可以在路由上设置一次回退值，不必逐个模型写：

```yaml
- id: llm-pi-ai
  config:
    providers:
      vision-gateway:
        apiKeyEnv: GATEWAY_API_KEY
        api: openai-completions
        baseURL: https://vision.example/v1
        defaultInput: [text, image]
        models:
          - id: first-model
          - id: second-model
```

`defaultInput` 是回退值而不是覆盖值，默认为 `[text]`：在内置提供商上，它只为其目录未描述的模型作答，因此绝不会把目录中本就具备图片能力的模型的该能力去掉。要收窄这类模型，请用它自己的 `input`。内置提供商没有显式 `models` 列表时，写在 `modelOverrides` 下，以模型 id 为键：

```yaml
- id: llm-pi-ai
  config:
    providers:
      anthropic:
        modelOverrides:
          claude-sonnet-4-5:
            input: [text]
```

在 pi-ai 配置中，除模型自身的 `input` 外，每个列表都至少要写一项模态；模型自身的空列表与省略它同义。未知模态在任何位置写入都会被拒绝。

这两个字段都是对你端点的断言，而不是对它的检查。声明了端点并不提供的图片能力的模型不会在这里被拦下，改由提供商拒绝该请求。

### DeepSeek 账号的模型

已登录 DeepSeek 账号的模型列表来自账号服务，因此模型页不为它提供模型编辑器。要调整单个模型，在 `$DSH_HOME/profiles/<profile>/cordis.patch.yml` 中使用 `modelOverrides`；字段与 DeepSeek 的 `models` 条目相同，未设置的字段沿用列表中的值：

```yaml
- id: llm-deepseek-account
  config:
    modelOverrides:
      deepseek-flash:
        contextWindow: 524288
        maxTokens: 131072
```

需要更新版 DeepSeek Harness 的模型会以 `CLIENT_UPGRADE_REQUIRED` 失败，并写明需要安装的版本。列表、兜底和门控规则见[账号提供方参考](../../../packages/llm/llm-deepseek-account/README.zh.md)。

### 推理等级

对于声明了推理等级的模型，模型选择器会提供**推理等级**菜单。内置提供商的模型从已安装目录继承其等级，DeepSeek 的模型沿用适配器提供的等级。手动录入的模型在添加等级之前不声明任何等级，因此模型菜单里不会出现推理等级项，由端点自身的默认值决定模型是否思考。

自定义模型 API 提供商可以在**设置 → 模型**中声明等级：编辑该提供商，打开**自定义设置**，展开该模型的**模型选项**，然后打开**推理等级**。默认带有 off、low、medium、high 四行。每行把一个档位与发送给它的 **API 参数值**配成一对，参数值默认与档位名相同；只有模型提供商的 API 文档写法不同时才需要修改。按端点支持的档位增删行，至少保留一个 off 以外的档位，然后保存。关闭开关会删除该声明。

模型页会把 `reasoningEfforts` 写入 `$DSH_HOME/profiles/<profile>/cordis.patch.yml`，也可以直接在其中声明等级：

```yaml
- id: llm-pi-ai
  config:
    providers:
      my-gateway:
        apiKeyEnv: GATEWAY_API_KEY
        api: openai-completions
        baseURL: https://gateway.example/v1
        reasoning: high
        models:
          - id: my-reasoner
            reasoningEfforts:
              off:
              high: high
              max: max
```

每个键都是菜单提供的一个等级，其值是在协议上以 `reasoning_effort` 发送的写法，因此 `max: xhigh` 可以为自有一套词汇的网关重命名某个等级。只有 `off` 可以留空，因为对多数端点来说，不思考就是不传该参数。路由的 `reasoning` 是会话尚未选择等级时采用的等级；在选择器中选定某个等级后，它会与模型一起保存为新会话的默认值。

留空的 `off` 什么都不发送，这只能让「按请求才思考」的模型停下来；给 `off` 一个值，则会把该值作为 `reasoning_effort` 发送。对于「不明确关闭就会思考」的模型——例如 OpenAI 兼容网关后面的 DeepSeek V4——需要 `compat.thinkingFormat: deepseek`：它让 `off` 发送 `thinking: {type: disabled}`，其他每个等级则在 effort 之外再发送 `thinking: {type: enabled}`：

```yaml
        models:
          - id: deepseek-v4-pro
            compat:
              thinkingFormat: deepseek
            reasoningEfforts:
              off:
              high: high
              max: max
```

网关并不提供推理能力的内置提供商模型，可在 `modelOverrides` 下用 `reasoningEfforts: false` 去掉其等级；之后再为它选择等级会被拒绝并报 `UNSUPPORTED_REASONING_EFFORT`。DeepSeek 自身的路由不需要以上任何配置：其模型已经提供 `off`、`low`、`high` 和 `max`，`llm-deepseek.reasoningEffort` 设置选择器的起始默认值：

```yaml
- id: llm-deepseek
  config:
    reasoningEffort: max
```

### 请求兼容性

网关可能持有可用的密钥、地址也通得到，却仍然拒绝每一个请求。pi-ai 依据端点的 URL 决定请求的形状——系统提示词由哪个角色承载、输出上限写在哪个字段、思考级别如何传输——而对于它无法识别的地址，会当作 OpenAI 本身来对待。多数 OpenAI 兼容网关至少会拒绝 OpenAI 所接受的某一样东西。

其中两样占了绝大多数。声明了推理能力的模型，其系统提示词会以 `role: "developer"` 发出，很多网关直接拒绝；输出上限则写作 `max_completion_tokens`，只认 `max_tokens` 的服务端会拒绝。表单里没有这两个字段；请在 `$DSH_HOME/profiles/<profile>/cordis.patch.yml` 的路由上更正：

```yaml
- id: llm-pi-ai
  config:
    providers:
      my-gateway:
        apiKeyEnv: GATEWAY_API_KEY
        api: openai-completions
        baseURL: https://gateway.example/v1
        compat:
          supportsDeveloperRole: false
          maxTokensField: max_tokens
        models:
          - id: my-model
```

路由的 `compat` 是其模型的默认值，模型自身的则逐字段胜出，因此更正某一个模型无需重述整条路由：

```yaml
        models:
          - id: my-model
          - id: my-reasoner
            compat:
              thinkingFormat: deepseek
```

两者都未设置的字段，沿用已安装 catalog 为该模型记录的值；catalog 也未描述的，落到 pi-ai 的检测。凡是写下的开关都要给值：冒号后留空的键（`supportsDeveloperRole:`）会被拒绝而不是被忽略，因为空值会抹掉 catalog 已知的信息，却又没有给出任何替代。任何协议都不接受的名字同样会被拒绝，报错会列出可用的那些。

每个开关归属于声明了它的那些协议，因此在某个 `api` 上合法的开关，在另一个上可能被拒绝——报错会点名该协议实际提供哪些。与上面的 `input` 一样，开关陈述的是关于你的端点的一个断言，而不是对它的检查：设置一个网关其实并不需要的开关，只是发出一个不同的请求而已。

全部开关、各自接受的取值，以及接受它们的协议，都列在[生成的 `dsh-llm-pi-ai` 配置参考](../../config-catalog.zh.md#deepseek-aidsh-llm-pi-ai)的 `PiAiCompatProfile` 之下——该参考派生自源码，因此不会落后于适配器实际接受的内容。

## 排错

- **`MISSING_CREDENTIAL`**：通过模型页存储提供商密钥，或提供被引用的环境变量。
- **`UNKNOWN_MODEL`**：选择已配置的模型，或向自定义提供商添加缺失的模型。
- **获取可用模型返回 401**：检查密钥。模型发现会调用 OpenAI 兼容的 `GET /models` 端点；对于不提供该端点的服务，请手动输入模型。
- **获取可用模型提示既没有 `data` 数组也没有 `models` 对象**：端点返回的列表格式不在探测的读取范围内。请手动输入模型。
- **密钥与地址都正确，网关却拒绝每一个请求**：它的请求形状与 OpenAI 不同。先在路由上设 `compat.supportsDeveloperRole: false` 与 `compat.maxTokensField: max_tokens`。
- **只有推理模型失败**：pi-ai 把它们的系统提示词以 `developer` 角色发出，而网关拒绝该角色。设 `compat.supportsDeveloperRole: false`。
- **手动录入的模型没有推理等级菜单**：该模型没有声明任何等级。如果是自定义模型 API 提供商，在该模型的**模型选项**中打开**推理等级**；其他提供商在 `cordis.patch.yml` 中给该模型加上 `reasoningEfforts`。
- **`off` 无法让 DeepSeek 模型停止思考**：留空的 `off` 不发送任何推理字段，默认思考的端点就继续思考。请在模型或路由上设置 `compat.thinkingFormat: deepseek`。
- **某个 compat 开关因没有值而被拒绝**：冒号后什么都没写。给它一个值，或删掉该键以沿用已安装 catalog 的值。
- **图片在发送前被拒绝**：该模型未声明图片模态。请给自定义提供商的模型加上 `input: [text, image]`；在 DeepSeek 自身的路由上，请从配置的目录中选择支持图片的条目（默认为 `deepseek-flash`），并确认网关提供该模型且支持图片输入。
- **提供商拒绝了带图片的请求**：该模型声明了其端点实际并不提供的图片能力。请从授予它图片能力的那个列表中移除 `image`——可能是模型的 `input`，也可能是路由的 `defaultInput`——然后开启新会话：附加的图片会留在会话日志里，因此在会话离开它之前，同一个请求会不断重复。
