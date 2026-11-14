# Toolchain and step-by-step

> 中文版：[06-toolchain.zh.md](06-toolchain.zh.md)

Everything the capture → merge → prewarm loop needs, with the versions used for the numbers in
[`03-results.md`](03-results.md). Nothing here requires game data to be committed; the dump is
yours and stays on your machine.

## A. What you need

| Piece | Version / source | Why |
|---|---|---|
| Apple Silicon Mac | M1 or later | builds the iOS app and runs the prepared data |
| macOS | 14 or later (26 for the numbers here) | Xcode 26 host |
| Xcode | 26.x, iOS 26 SDK | compiler, `xcodebuild`, `devicectl` |
| Metal Toolchain | `xcodebuild -downloadComponent MetalToolchain` | AOT shader compile, `metal-tt`, `metal-pack` |
| Rust | 1.98.1 + `aarch64-apple-ios` target | the emulator itself |
| Homebrew packages | `cmake`, `pkgconf`, `xcodegen` | build tooling (CI installs these) |
| Python | 3.10+ (tested 3.14.7) | capture pull / corpus merge |
| pymobiledevice3 | 11.51.0 | USB access to the app's Documents |
| iPhone / iPad | iOS 26+, 6 GB RAM or more | target device, Developer Mode on |
| Game dump | your own: `prod.keys`, `base.nsp`, `update.nsp` (4.0.1 or 3.0.5), optional `dlc.nsp` | the game is never part of this repo |

Notes:

- `make-ios.sh` pins `TOOLCHAIN=1.98.1`; the CI falls back to `stable` if that toolchain is not
  available. `cargo build --release` at the top of the script builds the host tools.
- `libvpx` is vendored under `crates/vpx`, so no extra Homebrew formula is needed for it.
- A free Apple ID works (AltStore / SideStore), with the documented limits: 7-day provisioning
  and no extended virtual addressing entitlement (5 GB layout).

## B. Install

```sh
# Xcode command line pieces
xcodebuild -version
xcodebuild -downloadComponent MetalToolchain
xcrun -sdk iphoneos metal --version      # must print a version

# Homebrew deps
brew install cmake pkgconf xcodegen

# Rust
rustup toolchain install 1.98.1
rustup target add aarch64-apple-ios --toolchain 1.98.1

# capture tooling (host side)
python3 -m pip install --user pymobiledevice3
python3 -c "import pymobiledevice3; print(pymobiledevice3.__version__)"
```

If the Metal Toolchain is missing, builds still succeed: shaders are compiled at runtime instead,
which is exactly the behaviour this document is about removing.

## C. Game files

Put your dump at the repository root (all of it is covered by `.gitignore`):

```
prod.keys      base.nsp      update.nsp      dlc.nsp (optional)
Firmware/      # fonts, Miis - needed by the macOS Prepare step
```

On iOS the app reads `Documents/prod.keys`, `Documents/base.nsp`, `Documents/update.nsp` (import
them through the Files app or `devicectl device copy`). `make-ios.sh` extracts the ExeFS modules
into `work/v-upd` on the Mac, so the phone does not need the `.nsp` files after installation.

For iPhones below the 15 Pro, add `--romfs` to rewrite BC1/BC3/BC7 textures as ASTC; the
resulting `work/prepared_ios.noindex/prepared` can be copied to the app instead of the `.nsp`s.

## D. Build

| Goal | Command | Output |
|---|---|---|
| macOS app | `./build.sh` | `~/Downloads/Mush8Output/Mush8.app` |
| macOS app (manual) | `./make-app.sh` | `build/Mush8.app` |
| iOS, straight to device | `./make-ios.sh --install` | installed `Mush8.app` |
| iOS, installable `.ipa` | `./make-ios.sh --ipa` | `build/Mush8.ipa` |
| install an `.ipa` | `./install-phone.sh --ipa` | installs on the connected device |

Environment knobs:

```sh
TEAM=<your Apple team id>        # otherwise read from Xcode > Settings > Accounts
BUNDLE_ID=com.example.mush8      # your own app id
FREE=1                           # entitlements a free account can sign
PSO_GPUS=applegpu_g14            # render Metal 4 pipeline archives for this GPU family
```

`PSO_GPUS` drives `metal-tt` + `metal-pack` (Metal Toolchain). The archives it produces are what
cut prewarm from 31.2 s to 7.0 s in the results; without them the same pipelines are compiled on
first launch instead. `xcrun -sdk iphoneos metal-tt -help` lists the accepted `-arch` names for
your Xcode version; use the one matching the target device.

The `.ipa` from `--ipa` is ad-hoc signed (no certificate needed). Install it with
`./install-phone.sh --ipa`, `xcrun devicectl device install app`, or AltStore / SideStore.

## E. Turn capture on

iOS: create `Documents/env.txt` (the app reads it at launch, one `KEY=VALUE` per line;
`Documents/` is shared with the Files app):

```
NX_DUMP_MSL=1
NX_RECORD_PIPELINES=1
NX_SLOW_FRAME=100
NX_PREWARM_THREADS=6
NX_FPS=1
```

macOS: export the same variables before launching the app.

What appears after a play session:

```
Documents/work/msl_rt/<hash>.bin     # original NVN shader blob  (goes into the corpus)
Documents/work/msl_rt/<hash>.metal   # MSL that was compiled     (for inspection)
Documents/data/pipelines.tsv         # every pipeline that was built
Documents/work/nxhost.log            # launch log; previous run kept as nxhost.prev.log
```

## F. Pull and merge

```sh
python3 tools/pull_shader_dumps.py --bundle <BUNDLE_ID> \
    --out dump_msl_rt --pipelines-out dump_work/pipelines_rt.tsv \
    --log dump_work/nxhost.log

python3 tools/merge_corpus.py --repo . \
    --capture dump_msl_rt --pipelines dump_work/pipelines_rt.tsv
```

The pull needs the device unlocked and trusted, Developer Mode on, and the app launched at least
once (that is when `Documents/` and `work/` are created). Re-running skips files already pulled,
so it is safe to pull after every session. `merge_corpus.py` prints how many blobs were added and
the resulting `corpus.zip` entry count.

## G. Rebuild and verify

```sh
./make-ios.sh --ipa && ./install-phone.sh --ipa

python3 tools/pull_shader_dumps.py --bundle <BUNDLE_ID> --out /tmp/x --log dump_work/nxhost.log
grep -E 'prewarmed|coverage|first frame waited|slow frame|pipeline bases' dump_work/nxhost.log
```

What each line means:

| Log line | Meaning |
|---|---|
| `prewarmed A/B recorded pipelines` | how much of the shipped pipeline list is AOT-ready |
| `prewarm coverage: M/B ... from N distinct vs/fs pairs` | what will still compile on first use |
| `first frame waited X s` | one-off prewarm cost at launch |
| `pipeline bases after prewarm: A from archives, C compiled` | archive hit rate; `C` are the bases this install compiles once |
| `slow frame N: ... (pipeline / base / specialize / shader compile)` | where the stall went |

Copy the expected shapes from [`03-results.md`](03-results.md). The loop is done when `slow frame`
disappears for content you have already played once and the coverage number stops moving.

## H. Measuring frames

- `NX_FPS=1` prints frame statistics every 600 frames, including GPU ms.
- `NX_SLOW_FRAME=100` (ms) prints a breakdown for every frame above the threshold, capped at 30
  reports per launch.
- For a controlled sample, sit on the title screen or a replay for 60-90 s and collect the
  `NX_FPS` lines; the round-2 sample was 91 frames (worst 79 ms, median 18 ms, GPU 6.6 ms).
- The counters are process-wide: prewarm worker threads contribute, which is the point - a slow
  frame can be waiting on work another thread is doing.

## I. CI route (optional)

`.github/workflows/build-ipa.yml` runs on `macos-26`, installs the same toolchain, and builds
with `BUNDLE_ID=<id> ./make-ios.sh --ipa`, uploading `Mush8-ipa` as an artifact:

```sh
gh run list --workflow build-ipa.yml
gh run download <run-id> -n Mush8-ipa
```

Caveat: the workflow expects the extracted modules in `work/v-upd/exefs/` (it neutralises the
dump-dependent steps and fakes `update.nsp`). On a fresh public clone there is no game data, so
the job fails at *Neutralise the steps that need the game dumps* - run the build locally, or add
your own extracted modules to the repository before enabling CI. Never commit them.

## J. Troubleshooting

| Symptom | Likely cause / fix |
|---|---|
| `unable to find utility metal` / no Metal toolchain | `xcodebuild -downloadComponent MetalToolchain`; builds still work with runtime compile |
| `No device found` when pulling | USB cable, unlock + Trust, Developer Mode, `python3 -m pymobiledevice3 usbmux list` |
| AFC `PERM_DENIED` / `Documents` missing | launch the app once; `Documents/` is created at first start |
| `sign into Xcode ... or set TEAM=` | Xcode > Settings > Accounts, or pass `TEAM=<id>` |
| free account stops working after a week | 7-day provisioning; rebuild/reinstall |
| coverage unchanged after a merge | `env.txt` not in `Documents/`, or app not relaunched; the launch log starts with `[host] iOS: files in ...` |
| prewarm takes ~30 s every launch | no pipeline archives for this GPU: build with `PSO_GPUS=...` or let the first launch write them |
| shaders compile at runtime despite a corpus | game/update changed: corpus keys are version-specific, re-capture |

## K. Versions used for the published numbers

macOS 26 runner, Xcode 26 with the iOS 26 SDK and Metal Toolchain, Rust 1.98.1,
Python 3.14.7 + pymobiledevice3 11.51.0, iPhone on iOS 26 (Metal 4), MK8D update 4.0.1.
