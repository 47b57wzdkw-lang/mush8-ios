# 工具链与分步操作

> English: [06-toolchain.md](06-toolchain.md)

采集 → 合并 → 预热 这个循环需要的一切，以及 [`03-results.zh.md`](03-results.zh.md) 里那些数字所用
的版本。这里没有任何一步需要把游戏数据提交进仓库；dump 是你自己的，只留在你机器上。

## A. 需要什么

| 组件 | 版本 / 来源 | 用途 |
|---|---|---|
| Apple Silicon Mac | M1 或更新 | 构建 iOS app、运行准备好的数据 |
| macOS | 14 或更新（这里的数字用 26） | Xcode 26 宿主 |
| Xcode | 26.x，iOS 26 SDK | 编译器、`xcodebuild`、`devicectl` |
| Metal Toolchain | `xcodebuild -downloadComponent MetalToolchain` | AOT 着色器编译、`metal-tt`、`metal-pack` |
| Rust | 1.98.1 + `aarch64-apple-ios` target | 模拟器本体 |
| Homebrew 包 | `cmake`、`pkgconf`、`xcodegen` | 构建工具（CI 会装这些） |
| Python | 3.10+（实测 3.14.7） | 拉取采集、合并语料 |
| pymobiledevice3 | 11.51.0 | 通过 USB 访问 app 的 Documents |
| iPhone / iPad | iOS 26+，6 GB 内存或更多 | 目标设备，打开 Developer Mode |
| 游戏 dump | 自备：`prod.keys`、`base.nsp`、`update.nsp`（4.0.1 或 3.0.5），可选 `dlc.nsp` | 游戏永远不在本仓库里 |

说明：

- `make-ios.sh` 固定 `TOOLCHAIN=1.98.1`；如果这个 toolchain 不可用，CI 会回落到 `stable`。脚本
  开头的 `cargo build --release` 负责构建 host 工具。
- `libvpx` 以 vendored 形式放在 `crates/vpx`，不需要额外的 Homebrew formula。
- 免费 Apple ID 可用（AltStore / SideStore），但有限制：7 天 provisioning，且没有 extended virtual
  addressing entitlement（5 GB 内存布局）。

## B. 安装

```sh
# Xcode 命令行组件
xcodebuild -version
xcodebuild -downloadComponent MetalToolchain
xcrun -sdk iphoneos metal --version      # 必须打印出版本号

# Homebrew 依赖
brew install cmake pkgconf xcodegen

# Rust
rustup toolchain install 1.98.1
rustup target add aarch64-apple-ios --toolchain 1.98.1

# 采集工具（host 侧）
python3 -m pip install --user pymobiledevice3
python3 -c "import pymobiledevice3; print(pymobiledevice3.__version__)"
```

如果缺 Metal Toolchain，构建仍会成功：着色器改成运行时编译，而那正是本文档要消除的行为。

## C. 游戏文件

把 dump 放到仓库根目录（全部被 `.gitignore` 覆盖）：

```
prod.keys      base.nsp      update.nsp      dlc.nsp（可选）
Firmware/      # 字体、Mii——macOS 的 Prepare 步骤需要
```

在 iOS 上，app 读 `Documents/prod.keys`、`Documents/base.nsp`、`Documents/update.nsp`（通过「文件」
app 导入，或用 `devicectl device copy`）。`make-ios.sh` 会在 Mac 上把 ExeFS 模块解到 `work/v-upd`，
所以安装之后手机不再需要那些 `.nsp` 文件。

15 Pro 以下的 iPhone，加 `--romfs` 把 BC1/BC3/BC7 纹理重写成 ASTC；产出的
`work/prepared_ios.noindex/prepared` 可以拷进 app，替代那堆 `.nsp`。

## D. 构建

| 目标 | 命令 | 产物 |
|---|---|---|
| macOS app | `./build.sh` | `~/Downloads/Mush8Output/Mush8.app` |
| macOS app（手工） | `./make-app.sh` | `build/Mush8.app` |
| iOS，直接装机 | `./make-ios.sh --install` | 装好的 `Mush8.app` |
| iOS，可安装的 `.ipa` | `./make-ios.sh --ipa` | `build/Mush8.ipa` |
| 安装 `.ipa` | `./install-phone.sh --ipa` | 装到已连接的设备 |

环境变量开关：

```sh
TEAM=<你的 Apple team id>        # 否则从 Xcode > Settings > Accounts 读取
BUNDLE_ID=com.example.mush8      # 你自己的 app id
FREE=1                           # 免费账号能签的 entitlements
PSO_GPUS=applegpu_g14            # 为该 GPU 家族渲染 Metal 4 管线档案
```

`PSO_GPUS` 驱动 `metal-tt` + `metal-pack`（Metal Toolchain）。它产出的档案就是结果里把预热从
31.2 s 降到 7.0 s 的东西；没有它们，同样的管线会在首次启动时现编。`xcrun -sdk iphoneos metal-tt
-help` 会列出你这份 Xcode 接受的 `-arch` 名称；用与目标设备匹配的那个。

`--ipa` 产出的 `.ipa` 是 ad-hoc 签名的（不需要证书）。用 `./install-phone.sh --ipa`、
`xcrun devicectl device install app`，或 AltStore / SideStore 安装。

## E. 打开采集

iOS：创建 `Documents/env.txt`（app 启动时读取，每行一个 `KEY=VALUE`；`Documents/` 与「文件」app
共享）：

```
NX_DUMP_MSL=1
NX_RECORD_PIPELINES=1
NX_SLOW_FRAME=100
NX_PREWARM_THREADS=6
NX_FPS=1
```

macOS：启动 app 前 export 同样的变量。

一段游玩之后会出现：

```
Documents/work/msl_rt/<hash>.bin     # 原始 NVN 着色器 blob（进语料）
Documents/work/msl_rt/<hash>.metal   # 被编译的 MSL（供检查）
Documents/data/pipelines.tsv         # 构建过的每条管线
Documents/work/nxhost.log            # 启动日志；上一次运行保留为 nxhost.prev.log
```

## F. 拉取并合并

```sh
python3 tools/pull_shader_dumps.py --bundle <BUNDLE_ID> \
    --out dump_msl_rt --pipelines-out dump_work/pipelines_rt.tsv \
    --log dump_work/nxhost.log

python3 tools/merge_corpus.py --repo . \
    --capture dump_msl_rt --pipelines dump_work/pipelines_rt.tsv
```

拉取需要设备解锁并已信任、Developer Mode 打开、app 至少启动过一次（`Documents/` 和 `work/` 是在
那时创建的）。重跑会跳过已经拉过的文件，所以每次游玩之后拉一次是安全的。`merge_corpus.py` 会打印
新增了多少 blob，以及最终的 `corpus.zip` 条目数。

## G. 重构建并验证

```sh
./make-ios.sh --ipa && ./install-phone.sh --ipa

python3 tools/pull_shader_dumps.py --bundle <BUNDLE_ID> --out /tmp/x --log dump_work/nxhost.log
grep -E 'prewarmed|coverage|first frame waited|slow frame|pipeline bases' dump_work/nxhost.log
```

每一行的含义：

| 日志行 | 含义 |
|---|---|
| `prewarmed A/B recorded pipelines` | 随包管线列表里有多少已经 AOT 就绪 |
| `prewarm coverage: M/B ... from N distinct vs/fs pairs` | 还有哪些会在首次用到时现编 |
| `first frame waited X s` | 启动时一次性的预热成本 |
| `pipeline bases after prewarm: A from archives, C compiled` | 档案命中率；`C` 是这次安装要现编一次的底座 |
| `slow frame N: ... (pipeline / base / specialize / shader compile)` | 卡顿花在了哪 |

预期形态对照 [`03-results.zh.md`](03-results.zh.md)。当你已经玩过一次的内容不再出现 `slow frame`、
覆盖率数字也不再变化时，循环就结束了。

## H. 测帧

- `NX_FPS=1` 每 600 帧打印一次帧统计，含 GPU ms。
- `NX_SLOW_FRAME=100`（ms）为每个超阈值的帧打印拆解，每次启动最多 30 条。
- 想要一次受控采样，就停在标题页或回放 60–90 s，收集 `NX_FPS` 行；第 2 轮的样本是 91 帧
  （最坏 79 ms，中位 18 ms，GPU 6.6 ms）。
- 计数器是进程级的：预热 worker 线程也会贡献，这正是重点——一个慢帧可能是在等别的线程的活儿。

## I. CI 路线（可选）

`.github/workflows/build-ipa.yml` 跑在 `macos-26` 上，安装同样的工具链，用
`BUNDLE_ID=<id> ./make-ios.sh --ipa` 构建，并把 `Mush8-ipa` 作为 artifact 上传：

```sh
gh run list --workflow build-ipa.yml
gh run download <run-id> -n Mush8-ipa
```

注意：该 workflow 期望解出来的模块在 `work/v-upd/exefs/`（它会把依赖 dump 的步骤中和掉，并伪造
`update.nsp`）。在全新的公开 clone 上没有游戏数据，job 会在 *Neutralise the steps that need the game
dumps* 处失败——要么在本地构建，要么在启用 CI 前把你解出来的模块加进仓库。永远不要提交它们。

## J. 故障排查

| 症状 | 可能原因 / 修复 |
|---|---|
| `unable to find utility metal` / 没有 Metal toolchain | `xcodebuild -downloadComponent MetalToolchain`；没有它构建仍可用运行时编译 |
| 拉取时 `No device found` | USB 线、解锁 + 信任、Developer Mode、`python3 -m pymobiledevice3 usbmux list` |
| AFC `PERM_DENIED` / `Documents` 不存在 | 先启动 app 一次；`Documents/` 在首次启动时创建 |
| `sign into Xcode ... or set TEAM=` | Xcode > Settings > Accounts，或传 `TEAM=<id>` |
| 免费账号一周后失效 | 7 天 provisioning；重构建/重装 |
| 合并后覆盖率没变 | `env.txt` 不在 `Documents/`，或 app 没重启；启动日志以 `[host] iOS: files in ...` 开头 |
| 每次启动预热都要 ~30 s | 这个 GPU 没有管线档案：用 `PSO_GPUS=...` 构建，或让首次启动写出来 |
| 有语料但着色器还是现编 | 游戏/更新变了：语料 key 与版本绑定，重新采集 |

## K. 公布数字所用的版本

macOS 26 runner，Xcode 26 配 iOS 26 SDK 与 Metal Toolchain，Rust 1.98.1，
Python 3.14.7 + pymobiledevice3 11.51.0，iOS 26（Metal 4）的 iPhone，MK8D 更新 4.0.1。
