## ADDED Requirements

### Requirement: 优先继承主页 API LLM
系统 SHALL 在章节翻译设置初始化时优先选择主页当前已加载、存在于可用 Provider 目录中的 API LLM，并同步其 Provider。

#### Scenario: 主页已加载 API 模型
- **WHEN** 用户在主页加载一个 API LLM 后进入章节翻译
- **THEN** 章节翻译默认选择同一个 Provider 和模型

#### Scenario: 配置加载主页模型
- **WHEN** 用户应用处理配置使主页加载对应 API LLM 后进入章节翻译
- **THEN** 章节翻译继承该配置加载到主页的模型

### Requirement: 安全回退和手动选择
系统 MUST 不把主页本地模型用于仅支持 API 的章节翻译，并 SHALL 在没有可继承模型时沿用原有章节模型选择或目录首项；初始化后用户 SHALL 能手动改选章节模型。

#### Scenario: 主页模型不可用于章节翻译
- **WHEN** 主页未加载模型、加载的是本地模型或该 API 模型不在可用目录中
- **THEN** 章节翻译使用自身已有选择或可用目录首项

#### Scenario: 用户手动改选
- **WHEN** 初始化继承完成后用户在章节翻译页选择其他模型
- **THEN** 系统保留用户的新选择且不立即强制切回主页模型
