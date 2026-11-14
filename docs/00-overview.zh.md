# 现状汇总：用真机采集把「首用现编译」的卡顿逐轮消掉

> 记录于 2026-11-14。仓库只含源码与工具，**不含任何游戏数据**（密钥、nsp、ExeFS 模块、着色器、管线档案）。

## 1. 问题

Mush8 把 Switch 的 NVN 图形 API 映射到 Metal。游戏着色器有两种归宿：

- 构建期扫描 ExeFS，翻译成 MSL 并编译进 AOT 语料；
- 没被扫到的，第一次用到时在**渲染线程**上现场翻译 + 编译。

iOS 没有 JIT，第二条路就是卡顿来源。首次装机实测：参考管线里 **83%（5851/7060）** 找不到 AOT 着色器。
最坏一帧 **14952 ms**，日志把时间拆得很清楚：

```
[gfx] slow frame 2150: 14952 ms from submit to commit
  (all threads: pipeline 7577 ms, base 7617 ms, specialize 0 ms, shader compile 7259 ms)
```

`pipeline ≈ base`、`specialize ≈ 0` 说明代价全在编译整条管线底座（Metal 编译器），
特化本身几乎免费——所以优化方向是把「编译」搬到装机后第一次启动，而不是减少特化。

## 2. 方法：让设备自己把缺的着色器教回来

1. **采集**：打开 `NX_DUMP_MSL=1`、`NX_RECORD_PIPELINES=1`。游戏每现编译一个着色器，就把 NVN blob（`.bin`）
   和翻译后的 MSL（`.metal`）写进 `Documents/work/msl_rt/`；每条构建过的管线（vs/fs key、targets、
   blend、color state…）追加进 `Documents/data/pipelines.tsv`。
2. **回灌**：`tools/merge_corpus.py` 按内容去重、冲突改名，重建 `data/corpus.zip`，合并管线记录。
3. **溯源**：`nxshader` 新增 `scan_paths`，扫描 yaz0/SARC（最多 4 层）后记录每个着色器来自哪个文件，
   用来判断「构建期扫描为什么漏了它」。
4. **预热**：启动后 6 个工作线程按记录列表构建管线，日志报告 `prewarmed A/B` 与缺失的 vs/fs 对数量；
   首帧等待预热完成。
5. **测量**：`BUILD_NS/BASE_NS/SPEC_NS/SHADER_NS` 四个纳秒计数器 + 慢帧日志（`NX_SLOW_FRAME`，默认 100 ms），
   每帧输出「管线/底座/特化/着色器编译」四段耗时，精确定位卡在哪。
6. **调参**：把 `BASE_WAIT` 从 1 s 降到 150 ms——等别的线程编译同一个底座，等不到就自己编，
   比在渲染线程上干等 1 s 更划算。
7. **重复**：玩到新内容 → 采到新着色器 → 再回灌 → 再装机，直到覆盖率收敛。

## 3. 三轮数据

| 指标 | 初始 | 第 1 轮回灌 | 第 2 轮回灌 |
|---|---|---|---|
| 记录管线 | 7060 | 7374 | 7497 |
| 预热成功 | 1209（17%） | 2844（39%） | 3152（42%） |
| 缺 AOT 着色器 | 5851（83%） | 4530（61%） | 4345（58%） |
| 预热耗时 | 0.2 s（全命中） | 31.2 s | 7.0 s |
| 首帧等待 | 0.2 s | 33.3 s（一次性） | 7.5 s（一次性） |
| 最坏帧 | 14952 ms | 7912 ms | 无 >100 ms（标题页采样最坏 79 ms） |

- 第 2 轮预热从 31.2 s 降到 7.0 s，是因为把设备上的 Metal 4 管线档案（`work/pso4`，1308 个底座）随包带上，
  只剩 171 个底座需要现编；下次启动应回到 ~0.2 s。
- 标题页采样 91 帧：最坏 79 ms，>50 ms 仅 1 帧，中位 18 ms，GPU 6.6 ms/帧。
- 语料从 655 条涨到 1841 条（本地 `data/corpus.zip`），设备端目前又积累了 1847 对 blob，待下一轮合并。

## 4. 还剩什么

- 覆盖率 58% 的缺口是**还没玩到的内容**（其他赛道/道具/特效），首次出现仍会现编译，表现为 100–600 ms 的偶发卡顿；
  这是「玩到 → 采到 → 回灌」的收敛过程，不是一次性开关。
- 管线档案与着色器语料都是从游戏数据派生的，**不能随仓库分发**；仓库只给方法和工具。
- 数据来自单台设备（iOS 26 / Metal 4），其它 GPU 家族需要各自采集一轮。

## 5. 工具与步骤（速查）

**工具**：Apple Silicon Mac + macOS 14+、Xcode 26（iOS 26 SDK）+ Metal Toolchain、Rust 1.98.1 + `aarch64-apple-ios`、Homebrew 的 cmake/pkgconf/xcodegen、Python 3.10+ 与 pymobiledevice3 11.51、iPhone（iOS 26+，6GB 内存）、自备游戏 dump。

**安装**：`xcodebuild -downloadComponent MetalToolchain`；`brew install cmake pkgconf xcodegen`；`rustup toolchain install 1.98.1 && rustup target add aarch64-apple-ios --toolchain 1.98.1`；`pip install pymobiledevice3`。

**构建**：`./make-ios.sh --ipa`（ad-hoc 签名；`BUNDLE_ID`/`TEAM`/`FREE=1` 可选；加 `PSO_GPUS=applegpu_g14` 可生成本机 GPU 的管线档案）→ `./install-phone.sh --ipa`。

**采集**：App 的 `Documents/env.txt` 写入 `NX_DUMP_MSL=1`、`NX_RECORD_PIPELINES=1`、`NX_SLOW_FRAME=100`、`NX_PREWARM_THREADS=6`、`NX_FPS=1`，重启 App 后正常玩一局。

**回灌**：

```sh
python3 tools/pull_shader_dumps.py --bundle <BUNDLE_ID> \
    --out dump_msl_rt --pipelines-out dump_work/pipelines_rt.tsv --log dump_work/nxhost.log
python3 tools/merge_corpus.py --repo . --capture dump_msl_rt --pipelines dump_work/pipelines_rt.tsv
```

**复测**：重新构建安装，`grep -E 'prewarmed|coverage|first frame waited|slow frame' dump_work/nxhost.log`。
完整版本、参数含义、CI 方式与故障排查见 [`06-toolchain.md`](06-toolchain.md)。
