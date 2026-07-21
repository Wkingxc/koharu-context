## 1. 真实准备进度

- [x] 1.1 将准备调度改为逐页补齐缺失 Artifact，并验证缓存页面与新页面均按真实完成数单调推进 | risk=high | evidence=focused
- [x] 1.2 调整运行页翻译阶段，使单批仅显示等待状态、分批仅显示批次状态，不展示虚假页数进度 | risk=medium | evidence=focused

## 2. 累积摘要历史

- [x] 2.1 将已确认摘要改为有序列表并贯穿请求、继续、重试和 SSE 事件，验证第三批及重试携带全部历史摘要 | risk=high | evidence=focused
- [x] 2.2 在批次审核区展示可折叠只读历史摘要和唯一可编辑的当前摘要，并验证继续后只提交当前编辑值 | risk=medium | evidence=focused
