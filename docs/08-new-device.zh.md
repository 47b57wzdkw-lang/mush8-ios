# 换 Mac / 换 iPhone：一次部署完成

> English: [08-new-device.md](08-new-device.md)

仓库只带代码和文档——322 个跟踪文件，没有游戏数据、没有采集的着色器、没有管线档案。新克隆下来
跑不起来，需要放回三样东西：游戏 dump、派生的语料（可选，但 42% 的 AOT 覆盖率靠它）、以及应用
`Documents` 里的数据。

## 0. 抹机前 10 分钟

旧 Mac 上拷走：

| 项目 | 为什么 |
|---|---|
| `prod.keys`、`base.nsp`、`update.nsp`、`dlc.nsp`、`Firmware/` | 游戏本体，永远不进 git |
| `data/corpus.zip`、`data/pipelines.tsv` | 采集到的着色器语料及其管线清单 |
| `data/pso4.zip` 或 `data/pso4/` | 设过 `PSO_GPUS` 时的 Metal 4 管线档案 |
| `work/pipelines.tsv` | 设备上记录的管线，已合并进清单 |

旧 iPhone 抹掉前：

| 项目 | 做法 |
|---|---|
| 着色器 dump | `python3 tools/pull_shader_dumps.py --bundle ch.cclerc.mush8 --out dump_msl_rt --pipelines-out dump_work/pipelines_rt.tsv` |
| `env.txt`、mods、Mii | 文件 App → 我的 iPhone → Mush8，或 `xcrun devicectl device copy from` |

没有语料也能构建：`make-ios.sh` 会回退到把 ExeFS 模块里的所有着色器预编译（`modules-only`），
结果正确，但覆盖率退回 17% 基线。

## 1. 新 Mac：一屏装完工具链

```sh
xcodebuild -version                          # 26.x
xcodebuild -downloadComponent MetalToolchain
xcrun -sdk iphoneos metal --version          # 必须打印版本号
brew install cmake pkgconf xcodegen
rustup toolchain install 1.98.1
rustup target add aarch64-apple-ios --toolchain 1.98.1
python3 -m pip install --user pymobiledevice3
```

没有 Metal Toolchain 也能构建成功，但所有着色器都在渲染线程上运行时编译——正是本仓库要消除的
那种卡顿。

## 2. 克隆，并把 upstream 设为只读

```sh
git clone https://github.com/47b57wzdkw-lang/mush8-ios.git
cd mush8-ios
git remote add upstream https://github.com/c22dev/mush8.git
git remote set-url --push upstream DISABLED
```

`upstream` 只作参考（c22dev 的 Mush8，GPL-3.0）；最后一行让误敲的 `git push upstream` 立刻失败。
这条设置写在 `.git/config` 里，**不会**随 clone 带过去，每台新机器都要再执行一次。

## 3. 放回游戏文件

放在仓库根目录，以下全部在 `.gitignore` 里：

```sh
prod.keys  base.nsp  update.nsp  dlc.nsp  Firmware/
```

再把第 0 步的语料放回来：

```sh
data/corpus.zip  data/pipelines.tsv  data/pso4.zip
```

## 4. 构建

```sh
./make-ios.sh --ipa                        # build/Mush8.ipa，ad-hoc 签名，不需要证书
TEAM=XXXXXXXXXX ./make-ios.sh --install    # 签名并直接装到连接的设备
PSO_GPUS=applegpu_g14 ./make-ios.sh --ipa  # Metal 4 管线档案：预热 31.2 s -> 7.0 s
FREE=1 ./make-ios.sh --ipa                 # 免费 Apple ID 能签的 entitlements
```

`TEAM` 不设时从 Xcode → Settings → Accounts 读取；`BUNDLE_ID` 覆盖 `ch.cclerc.mush8`。
首次构建要编译宿主工具、解包 ExeFS 模块、为 iOS 预编译全部着色器——需要几分钟，进度在
`work/precompile_ios.log`。

iPhone 15 Pro 以下的机型加 `--romfs`：BC1/BC3/BC7 纹理转成 ASTC，`work/prepared_ios.noindex/prepared`
可以拷进应用，替代 `.nsp` 文件。

## 5. 新 iPhone

1. iOS 26 或更高，内存 6 GB 以上。
2. 设置 → 隐私与安全性 → 开发者模式，重启后信任这台 Mac。
3. 安装：`./make-ios.sh --install`、`xcrun devicectl device install app --device <id>
   build/Mush8.ipa`，或用 AltStore / SideStore 装 `.ipa`。
4. 拷数据：`./install-phone.sh` 会做——`prod.keys`、`Firmware/*`，然后按机型选 `prepared/*`
   （15 Pro 以下）或 `base.nsp` + `update.nsp` + `dlc.nsp`（8 GB 机型）。已存在且大小相同的
   文件会跳过。
5. `env.txt` 任何脚本都不拷：需要采集或 FPS 日志时，在文件 App 里新建 `Documents/env.txt`。

`install-phone.sh` 只接受 `--nsp` 和可选的设备 ID，没有 `--ipa` 参数；`.ipa` 交给 `devicectl`
或 AltStore。

免费 Apple ID 是 7 天签名（每周重签），且没有扩展虚拟寻址权限，游戏跑在 5 GB 内存布局下。

## 6. 验证

```sh
grep -E 'prewarmed|coverage|first frame waited|slow frame' nxhost.log
```

用文件 App 或 `devicectl device copy from` 取回 `Documents/work/nxhost.log`。想要的形态：
`prewarmed` 上升、`coverage` 的缺失数下降、`first frame waited` 在设备档案补齐后稳定到约 0.2 s、
已经玩过一次的内容不再出现 `slow frame`。

## 7. 新机器常见坑

| 现象 | 原因 |
|---|---|
| `xcrun: error: unable to find utility 'metal'` | 没有下载 Metal Toolchain 组件 |
| `sign into Xcode ...` | Xcode → Settings → Accounts 里没有账号，或没设 `TEAM` |
| `no iPhone / iPad connected` | 设备锁屏、没开开发者模式，或没信任这台 Mac |
| `fatal: 'DISABLED' does not appear to be a git repository` | 正常，这就是防误推 |
| `xcodegen: command not found` | `brew install xcodegen`；`.xcodeproj` 由 `make-ios.sh` 生成 |
| 每次构建都重编着色器 | `base.nsp`/`update.nsp` 大小变了，或 `data/corpus.zip` 不在 |
