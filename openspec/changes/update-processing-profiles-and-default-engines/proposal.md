## Why

已保存的处理配置目前只能新增、应用和删除，用户修改当前设置后无法覆盖更新原配置；同时首次安装的流水线默认引擎与已验证的漫画处理组合不一致，需要手动逐项调整。

## What Changes

- 在顶部“配置”菜单中增加“更新当前配置”，用当前所有可保存设置覆盖当前选中的配置。
- 未选中配置时禁用更新入口，避免误更新其他配置。
- 将全新配置的八个流水线阶段默认值固定为指定的漫画检测、字体检测、分割、OCR、LLM 翻译、修补与渲染引擎组合。
- 已有安装中已经持久化的引擎选择保持不变。

## Capabilities

### New Capabilities
- `processing-profile-update`: 覆盖更新当前选中处理配置的交互与持久化行为。
- `default-pipeline-engines`: 全新安装或缺失流水线配置时使用的默认引擎组合。

### Modified Capabilities

## Impact

影响 `ui/lib/stores/processingProfileStore.ts`、顶部菜单及其中英文文案和测试，以及 `crates/koharu-app/src/config.rs` 的流水线默认配置与测试；不改变 API Key、数据目录或既有用户配置的迁移规则。
