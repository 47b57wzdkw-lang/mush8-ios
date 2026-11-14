# New Mac, new iPhone: redeploy in one pass

> 中文版：[08-new-device.zh.md](08-new-device.zh.md)

This repository ships code and notes only — 322 tracked files, no game data, no captured shaders,
no pipeline archives. A fresh clone has nothing runnable until three things are put back: the game
dump, the derived corpus (optional, but it is where the 42% AOT coverage comes from), and the
app's `Documents` payload.

## 0. Before wiping the old machine (10 minutes)

Copy these off the old Mac:

| Item | Why |
|---|---|
| `prod.keys`, `base.nsp`, `update.nsp`, `dlc.nsp`, `Firmware/` | the game itself; never in git |
| `data/corpus.zip`, `data/pipelines.tsv` | captured shader corpus and its pipeline list |
| `data/pso4.zip` or `data/pso4/` | Metal 4 pipeline archives, when `PSO_GPUS` was used |
| `work/pipelines.tsv` | pipelines recorded on the device, merged into the list |

And off the old iPhone, before erasing it:

| Item | How |
|---|---|
| shader dumps | `python3 tools/pull_shader_dumps.py --bundle ch.cclerc.mush8 --out dump_msl_rt --pipelines-out dump_work/pipelines_rt.tsv` |
| `env.txt`, mods, Miis | Files app → On My iPhone → Mush8, or `xcrun devicectl device copy from` |

Skip the corpus and the build still works: `make-ios.sh` falls back to precompiling every shader
the ExeFS modules carry (`modules-only`), which is correct but back to the 17% baseline.

## 1. New Mac: toolchain in one screen

```sh
xcodebuild -version                          # 26.x
xcodebuild -downloadComponent MetalToolchain
xcrun -sdk iphoneos metal --version          # must print a version
brew install cmake pkgconf xcodegen
rustup toolchain install 1.98.1
rustup target add aarch64-apple-ios --toolchain 1.98.1
python3 -m pip install --user pymobiledevice3
```

Without the Metal Toolchain the build still succeeds, but every shader is compiled at runtime on
the render thread — exactly the stall this repository is about.

## 2. Clone, and make upstream read-only

```sh
git clone https://github.com/47b57wzdkw-lang/mush8-ios.git
cd mush8-ios
git remote add upstream https://github.com/c22dev/mush8.git
git remote set-url --push upstream DISABLED
```

`upstream` is a reference only (Mush8 by c22dev, GPL-3.0); the last line makes an accidental
`git push upstream` fail immediately. The setting lives in `.git/config` and is **not** carried
by a clone — run it again on every new machine.

## 3. Put the game files back

At the repository root; everything below is in `.gitignore`:

```sh
prod.keys  base.nsp  update.nsp  dlc.nsp  Firmware/
```

Then restore the corpus from step 0:

```sh
data/corpus.zip  data/pipelines.tsv  data/pso4.zip
```

## 4. Build

```sh
./make-ios.sh --ipa                        # build/Mush8.ipa, ad-hoc signed, no certificate
TEAM=XXXXXXXXXX ./make-ios.sh --install    # sign and install on the connected device
PSO_GPUS=applegpu_g14 ./make-ios.sh --ipa  # Metal 4 pipeline archives: prewarm 31.2 s -> 7.0 s
FREE=1 ./make-ios.sh --ipa                 # entitlements a free Apple ID can sign
```

`TEAM` is otherwise read from Xcode → Settings → Accounts; `BUNDLE_ID` overrides
`ch.cclerc.mush8`. The first build compiles the host tools, extracts the ExeFS modules and
precompiles every shader for iOS — minutes, with progress in `work/precompile_ios.log`.

For iPhones below the 15 Pro add `--romfs`: BC1/BC3/BC7 textures become ASTC, and
`work/prepared_ios.noindex/prepared` can be copied to the app instead of the `.nsp` files.

## 5. New iPhone

1. iOS 26 or later, 6 GB RAM or more.
2. Settings → Privacy & Security → Developer Mode, reboot, trust the Mac.
3. Install: `./make-ios.sh --install`, `xcrun devicectl device install app --device <id>
   build/Mush8.ipa`, or AltStore / SideStore for the `.ipa`.
4. Copy the data: `./install-phone.sh` does it — `prod.keys`, `Firmware/*`, then either
   `prepared/*` (devices below 15 Pro) or `base.nsp` + `update.nsp` + `dlc.nsp` (8 GB devices).
   Files already present at the same size are skipped.
5. `env.txt` is not copied by any script: create `Documents/env.txt` in the Files app when you
   want capture or FPS logging.

`install-phone.sh` accepts only `--nsp` and an optional device id — there is no `--ipa` flag;
pass the `.ipa` to `devicectl` or AltStore instead.

A free Apple ID gets 7-day provisioning (re-sign weekly) and no extended virtual addressing
entitlement, so the game runs with the 5 GB layout.

## 6. Verify

```sh
grep -E 'prewarmed|coverage|first frame waited|slow frame' nxhost.log
```

Pull `Documents/work/nxhost.log` with the Files app or `devicectl device copy from`. The shape
you want back: `prewarmed` rising, the `coverage` missing count falling, `first frame waited`
settling to ~0.2 s once the device archives are populated, `slow frame` gone for content already
played once.

## 7. Fresh-machine pitfalls

| Symptom | Cause |
|---|---|
| `xcrun: error: unable to find utility 'metal'` | Metal Toolchain component not downloaded |
| `sign into Xcode ...` | no Apple ID in Xcode → Settings → Accounts, or `TEAM` unset |
| `no iPhone / iPad connected` | device locked, Developer Mode off, or Mac not trusted |
| `fatal: 'DISABLED' does not appear to be a git repository` | expected: that is the push guard |
| `xcodegen: command not found` | `brew install xcodegen`; the `.xcodeproj` is generated by `make-ios.sh` |
| shaders recompiled on every build | `base.nsp`/`update.nsp` size changed, or `data/corpus.zip` missing |
