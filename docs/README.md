# Mush8 iOS — runtime shader corpus, AOT prewarm and the numbers behind it

Mush8 translates the Switch's NVN graphics API to Metal. On iOS there is **no JIT**, so a
pipeline whose GLSL/MSL has not been precompiled at build time is translated and compiled by the
Metal compiler **on the render thread, on device**, the first time a scene uses it. That is the
hitch — and occasionally a multiple-second freeze — you see when a race starts.

This repository is the iOS port of Mush8 plus the capture → merge → prewarm loop that removes
those stalls by learning from the device itself. It ships **no game data**: no keys, no `.nsp`,
no extracted ExeFS modules, no captured shaders, no pipeline archives.

## Documents

| Document | What it covers |
|---|---|
| [`00-overview.md`](00-overview.md) | English summary: problem, method, three rounds, limits |
| [`00-overview.zh.md`](00-overview.zh.md) | 中文汇总：背景、方法、三轮实测数据、边界 |
| [`01-diagnosis.md`](01-diagnosis.md) | Why the stalls happen, and the evidence that pinned them |
| [`01-diagnosis.zh.md`](01-diagnosis.zh.md) | 诊断：卡顿从哪来、埋点、基线、只扫 ExeFS 为何不够 |
| [`02-method.md`](02-method.md) | Capture hooks, corpus merge, `shfind`, prewarm, instrumentation |
| [`02-method.zh.md`](02-method.zh.md) | 方法：采集、合并、溯源、预热、管线档案、调参 |
| [`03-results.md`](03-results.md) | Three rounds of measurements, before/after |
| [`03-results.zh.md`](03-results.zh.md) | 结果：覆盖率、启动、最坏帧、语料增长 |
| [`04-reproduce.md`](04-reproduce.md) | Build, capture, merge, measure — short version |
| [`04-reproduce.zh.md`](04-reproduce.zh.md) | 复现：构建、采集、合并、验证、着色器溯源 |
| [`05-limitations.md`](05-limitations.md) | What is still not covered, and why |
| [`05-limitations.zh.md`](05-limitations.zh.md) | 局限与未解问题 |
| [`06-toolchain.md`](06-toolchain.md) | Full toolchain, versions, build/capture/merge steps, troubleshooting |
| [`06-toolchain.zh.md`](06-toolchain.zh.md) | 工具链、版本、分步操作与故障排查 |
| [`07-publish.md`](07-publish.md) | Publishing flow, the 11 failures it hit, and each fix |
| [`07-publish.zh.md`](07-publish.zh.md) | 发布流程与踩过的 11 个坑 |
| [`08-new-device.md`](08-new-device.md) | New Mac / new iPhone: what to back up, and redeploy in one pass |
| [`08-new-device.zh.md`](08-new-device.zh.md) | 换 Mac / 换 iPhone：备份清单与快速重新部署 |

## Headline numbers

| Metric | Before | After round 2 |
|---|---|---|
| Recorded pipelines with AOT shaders | 1209 / 7060 (17%) | 3152 / 7497 (42%) |
| Missing AOT shaders | 5851 (83%) | 4345 (58%) |
| Worst frame observed | 14952 ms | no frame >100 ms (worst sample 79 ms) |
| First-frame wait | 0.2 s | 7.5 s once, then ~0.2 s from device archives |

## Upstream

Mush8 by Constantin Clerc (c22dev): <https://github.com/c22dev/mush8> (GPL-3.0).
This fork keeps the licence and adds the iOS build path and the corpus loop in `docs/`.