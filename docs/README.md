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
| [`02-method.md`](02-method.md) | Capture hooks, corpus merge, `shfind`, prewarm, instrumentation |
| [`03-results.md`](03-results.md) | Three rounds of measurements, before/after |
| [`04-reproduce.md`](04-reproduce.md) | Build, capture, merge, measure — step by step |
| [`05-limitations.md`](05-limitations.md) | What is still not covered, and why |

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