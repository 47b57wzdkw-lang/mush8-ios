# 诊断：卡顿从哪来

## 一段话讲清渲染模型

guest 游戏发出 NVN 命令，`nxhost` 把它们变成 Metal：NVN 着色器 blob 先翻译成 MSL，再编译——
构建期编进 AOT 语料，或者运行时在渲染线程上现编。`MTLRenderPipelineState` 由 *base*（vertex+fragment
程序）和 *specializations*（target formats、blend state、colour mask……）构成。base 的构建才是 Metal
编译器跑的地方；特化很便宜。

iOS 上没有 JIT，也没有权限去 spawn 一个能廉价地在别的线程上编译的进程，所以语料里缺的任何东西
都会在提交帧的时候被同步编译。

## 埋点

`crates/nxhost/src/nvn/draw/state.rs` 维护四个进程级纳秒计数器——`BUILD_NS`、`BASE_NS`、`SPEC_NS`、
`SHADER_NS`——分别由 `build_pipeline`、`Pso4::base`、`Pso4::pipeline` 和 `shaders::compile` 喂入。
`submit_now` 每帧采样，当某帧超过 `NX_SLOW_FRAME` ms（默认 100）时打印时间花在了哪：

```
[gfx] slow frame 2150: 14952 ms from submit to commit
  (all threads: pipeline 7577 ms, base 7617 ms, specialize 0 ms, shader compile 7259 ms)
```

计数器故意做成进程级的：管线也会被预热 worker 线程构建，一个慢帧会把它们的活儿继承进来。

## 基线（首次 iOS 装机）

```
[gfx] prewarmed 1209/7060 recorded pipelines in 0.2 s on 6 workers
[gfx] pipeline bases after prewarm: 503 from archives, 0 compiled, 1209 specializations, 0 failed
[gfx] first frame waited 0.2 s for the pipeline prewarm
```

只有 17% 的记录管线带 AOT 着色器；5851 条管线（来自 2719 个不同的 vs/fs 对）必须在首次用到时编译。
一场比赛开局因此产生了 14.9 s 的一帧：

- `pipeline ≈ base` → 代价在构建 base 程序，也就是 Metal 编译器；
- `specialize ≈ 0` → 特化不是问题；
- `shader compile ≈ 7.3 s` → 连那一个程序的 MSL 编译都要几秒。

第二次会话又在另一条管线上撞到 7912 ms（pipeline 3344 ms、base 3359 ms、shader compile 4499 ms）。
这两者都是构建期 ExeFS 扫描看不见的内容，因为管线变体是在运行时、用构建期从未执行到的状态拼出来的。

## 为什么只扫 ExeFS 不够

`nxshader::corpus::scan` 遍历游戏模块，解 yaz0 和最多四层 SARC，索引它能找到的每个 NVN 着色器 blob
（key = 代码段的 FNV-1a）。这只覆盖作为数据随包发布的着色器，不覆盖：

- 着色器对存在、但它的*变体*从未被记录的管线（target formats、额外输出、整数 target）；
- 运行时生成或打补丁的着色器；
- 这个分支从未走到的代码路径（赛道、模式、DLC）。

`state::prewarm` 里加的覆盖率报告把这件事精确量化了：`missing/total recorded pipelines have no AOT
shader, from N distinct vs/fs pairs`。这个数字就是采集循环要压下去的目标。
