## ADDED Requirements

### Requirement: API Key 本地文件持久化

系统 SHALL 把 Provider API Key 保存到当前 `DataConfig.path` 下的专用本地文件，而不是操作系统钥匙串。该文件 MUST 不被 Git 跟踪，配置 API 和日志 MUST 继续只返回脱敏值。

#### Scenario: 保存并重新加载 API Key

- **WHEN** 用户保存 Provider API Key 并重启应用
- **THEN** 系统从当前数据目录的本地私密文件加载该 Key，且不触发系统钥匙串授权

#### Scenario: 数据目录发生变更

- **WHEN** 用户应用新的 `DataConfig.path` 并重启应用
- **THEN** 系统在新数据目录保存和读取 API Key，同时 `config.toml` 继续保存用户选择的数据路径

### Requirement: 本地密钥文件最小暴露

系统 MUST 在支持文件权限的系统上以仅当前用户可读写的权限创建密钥文件，并 MUST 避免将真实 Key 写入 `config.toml`、配置预设、错误信息或日志。

#### Scenario: 检查持久化内容

- **WHEN** 系统保存包含 API Key 的配置
- **THEN** 专用密钥文件包含真实值，而 `config.toml` 和配置接口只包含 `[REDACTED]` 或空值
