# Migi — 共生型 AI Agent 规范索引

## 规范文件

| 规范 | 路径 | 描述 |
|------|------|------|
| 共生系统核心 | [symbiosis.spec.md](core/symbiosis.spec.md) | 四层架构、阶段控制、相变机制 |
| 感知层 | [observation.spec.md](core/observation.spec.md) | 观察通道、事件结构、日志/指标观察者 |
| 认知层 | [learning.spec.md](core/learning.spec.md) | 系统模型、预测生成、异常检测 |
| 行动层 | [intervention.spec.md](core/intervention.spec.md) | 动作类型、策略抽象、回滚机制 |
| 信任层 | [trust.spec.md](core/trust.spec.md) | 信任状态、授权检查、相变评估 |

## 变更跟踪

变更记录存放在 `changes/` 目录下。

## 如何使用

### 给 Claude Code

```
Implement the OpenSpec requirements in openspec/core/symbiosis.spec.md.
All code must follow the specifications. Write tests for each scenario.
```

### 给开发者

1. 阅读相关规范
2. 确认当前实现与规范的差距
3. 实现 → 测试 → clippy → commit
4. 如果规范需要修改，先写 spec delta
