# 复现

> English: [04-reproduce.md](04-reproduce.md)

你需要自己的游戏 dump（见 `README.md`）。本仓库不附带任何游戏数据、密钥或解出来的模块，所以纯
`git clone` **不会**得到能跑的 app，除非你自己把这些加进去。

## 1. 构建

```sh
./build.sh                 # macOS
./make-ios.sh --ipa        # iOS，产出 .ipa（之后再签名）
./make-ios.sh --install    # iOS，直接装到已连接的设备
```

`make-ios.sh` 会把游戏模块解出来、打补丁放进 `work/`，扫描其中的着色器，并预编译一份语料。没有
`data/corpus.zip` 时，它仍会预编译模块自带的所有着色器——这就是文档里的兜底路径。

## 2. 打开采集

把这些行写进 app 的 `Documents/env.txt`（iOS）或环境变量（macOS）：

```
NX_DUMP_MSL=1
NX_RECORD_PIPELINES=1
NX_SLOW_FRAME=100
NX_PREWARM_THREADS=6
```

把你在乎的内容完整玩一遍。设备会写：

- `Documents/work/msl_rt/<hash>.bin` 与 `.metal` —— 它不得不编译的每个着色器；
- `Documents/data/pipelines.tsv` —— 它构建过的每条管线。

## 3. 拉取并合并

```sh
# iOS：用 pymobiledevice3 + house_arrest
python3 tools/pull_shader_dumps.py --bundle ch.cclerc.mush8 \
    --out dump_msl_rt --pipelines-out dump_work/pipelines_rt.tsv

python3 tools/merge_corpus.py --repo . \
    --capture dump_msl_rt --pipelines dump_work/pipelines_rt.tsv
```

`merge_corpus.py` 会重建 `data/corpus.zip` 并就地合并 `data/pipelines.tsv`。两者都被 `.gitignore`
忽略：它们派生自游戏数据，不能提交。

## 4. 重构建、重装、验证

```sh
./make-ios.sh --ipa && ./install-phone.sh --ipa
```

然后看日志：

```sh
adb shell  # iOS 上不适用；用日志控制台，或把 Documents/work/nxhost.log 拉下来
grep -E 'prewarmed|coverage|first frame waited|slow frame' nxhost.log
```

预期形态：`prewarmed` 上升，`coverage` 的缺失数下降，设备档案齐了之后 `first frame waited` 稳定下来，
已经玩过一次的内容不再出现 `slow frame` 行。

## 5. 着色器溯源（可选）

```sh
# 某个 key 到底来自哪个模块/档案？
cargo run --release -p nxshader --bin shfind -- <exefs-dir> | head
```

`shfind` 为每个能溯源的着色器打印 `key<TAB>origin::member`，是查「缺失的 key 到底在不在随包数据里」
最快的办法。
