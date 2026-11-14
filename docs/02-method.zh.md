# 方法：真机采集、离线合并、启动预热

> English: [02-method.md](02-method.md)

这个循环刻意做得很无聊：设备告诉我们它不得不编译哪些着色器，我们把它们加进语料，重构建，再看
计数器有没有动。一切都由环境变量驱动，所以普通构建保持干净。

## 1. 采集（`NX_DUMP_MSL`、`NX_RECORD_PIPELINES`）

`crates/nxhost/src/nvn/draw/shaders.rs` 的 `dump_requested` 会给每个在设备上翻译的着色器，在
`Documents/work/msl_rt/` 下写一对文件：

- `<hash>.bin` —— 原始 NVN 着色器 blob（语料里存的正是它）；
- `<hash>.metal` —— 被编译的 MSL，供检查。

`state::record` 把每条构建过的管线（vs/fs key、targets、colour state、channel mask、blend）追加到
`Documents/data/pipelines.tsv`。两条路径都是 app 相对路径，所以文件能扛住 app 重启，也能通过
AFC/house_arrest 拉出来。

## 2. 合并（`tools/merge_corpus.py`）

`data/corpus.zip` 是内容寻址的：条目就是原始 NVN blob，按 SHA-1 去重，时间戳固定所以重建可复现。
如果采到的 blob 与已有条目同名但字节不同，就存成 `<name>_rt.bin`，而不是覆盖。`data/pipelines.tsv`
的行按集合合并（整行去重）。

## 3. 溯源（`nxshader` 的 `scan_paths`、`shfind`）

`corpus.rs` 现在只有一个 `blobs()` 遍历器，位置扫描和新的 `scan_paths()` 共用；后者记录每个着色器的
来源（`module::sarc/member`，最多四层 SARC）。`crates/nxshader/src/bin/shfind.rs` 把它暴露成 CLI。
这回答了「构建期扫描为什么漏了这个 key」——它到底是不存在、打包方式不同，还是压根不在任何随包档案里。

## 4. 预热（`state::prewarm`）

在第一个提交帧上 `gfx` 调 `state::wait_prewarm`，用 `NX_PREWARM_THREADS` 个工作线程（默认：可用 CPU）
构建每条记录管线：

```
[gfx] prewarmed 3152/7497 recorded pipelines in 7.0 s on 6 workers
[gfx] prewarm coverage: 4345/7497 recorded pipelines have no AOT shader,
      from 2579 distinct vs/fs pairs; those compile on first use
```

首帧会等预热完成（按设计有界：`NX_PREWARM_WAIT=0` 关掉等待）。

## 5. Metal 4 管线档案（`work/pso4`，设备生成）

在 Metal 4 设备上，驱动可以把编译好的管线底座序列化进一个档案文件。把这些（`PipelineBinaries`）采
下来、随 app 一起装，下次启动就能完全跳过编译器：`pipeline bases after prewarm: 1308 from archives,
171 compiled`——这就是两轮之间预热从 31.2 s 降到 7.0 s 的原因。这些档案与设备/驱动相关，因此
**不在**本仓库里。

## 6. 从数字里得出的调参

- `BASE_WAIT` 1 s → 150 ms（`pso4.rs`）：一次构建会等另一个线程编同一个底座；等待只是抄近路，
  而过长的 deadline 只会白白拖住渲染线程。
- 慢帧日志上限 30 条，游戏中埋点保持廉价。
- 预热线程数可调（`NX_PREWARM_THREADS`）；在测试机上，6 个线程是编译器饱和、又不会饿死游戏
  主线程的位置。

## 7. 循环

1. 装上开了采集的构建。
2. 把你在乎的内容（赛道、模式、DLC）完整玩一遍。
3. 拉 `msl_rt/` 和 `data/pipelines.tsv`，合并，重构建，重装。
4. 看 `prewarm coverage` 降了没有，重启，确认最坏帧和首帧等待。
5. 重复，直到覆盖率的平台期与你实际玩的内容对得上。
