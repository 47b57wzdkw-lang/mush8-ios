# 发布这个仓库：完整流程与踩过的每一个坑

记录 2026-11-14 从 **Windows** 主机把本仓库（iOS 移植源码 + 学习笔记）推上 GitHub 的全过程，
以及途中 11 个失败点和每一个的确切修法。不含任何游戏数据，普通 GitHub 账号即可复现。

> 配套章节：[`06-toolchain.md`](06-toolchain.md) 讲构建/采集/合并工具链及其故障排查，
> 本文只讲**发布**。

## A. 需要什么

| 组件 | 本次版本 | 说明 |
|---|---|---|
| Windows 主机 | Windows 11 2H2 | 任意较新 Windows 均可 |
| Git for Windows | 2.x（`git version`） | 自带 libcurl/c-ares，解析行为与系统不同 |
| PowerShell | 7.x（`pwsh`） | 所有 REST 调用都走它 |
| GitHub CLI | 2.104.0（`winget install GitHub.cli`） | 可选，见 P4 |
| 本地 HTTP 代理 | `127.0.0.1:7890`（Clash） | 仅网络受限时需要 |
| GitHub 账号 | `47b57wzdkw-lang` | 仓库所有者 |

仓库本身只是普通 git 仓库，发布不需要 Xcode 或 Mac。

## B. 六步流程

```text
1. 先看会提交什么        git status --porcelain ; git ls-files | wc -l
2. 建空仓库              POST /user/repos（name、private=false）
3. 拿 token              OAuth 设备流，或复用已有 token
4. 推送                  git push <token-url> main:main
5. 校验                  git ls-remote == git rev-parse HEAD
6. 校验渲染与元数据      API /repos、raw README.md
```

本仓库的真实数字：**320 个受跟踪文件**、**6 个提交**、HEAD
`e9970139dd312d7caff0da5c506d8fe9af2f7a9a`、默认分支 `main`。

### 1. 先检查工作区

```sh
git -C <repo> status --porcelain    # 必须为空（或只剩明知可忽略的文件）
git -C <repo> ls-files | wc -l      # 320
git -C <repo> log --oneline -6
```

仓库**不含** `prod.keys`、`.nsp`、解包出的 ExeFS 模块、采集到的着色器与管线归档，这些都进了
`.gitignore`。用下面的命令确认：

```sh
git -C <repo> ls-files | grep -Ei 'nsp|prod\.keys|exefs|\.zip$'   # 必须无输出
```

### 2. 建空仓库

API 方式（本次实际使用）：

```powershell
$h = @{ Authorization = "token $TOKEN"; Accept = 'application/vnd.github+json'; 'User-Agent' = 'notes' }
Invoke-RestMethod -Method Post -Uri 'https://api.github.com/user/repos' -Headers $h `
  -Body (@{ name='mush8-ios'; private=$false } | ConvertTo-Json -Compress) -ContentType 'application/json'
```

CLI 方式（token 具备 `gh` 要求的 scope 时，见 P4）：

```sh
gh repo create <user>/mush8-ios --public --source=<repo> --remote=origin --push
```

### 3. 拿 token（设备流）

申请授权码：

```powershell
$r = Invoke-RestMethod -Method Post -Uri 'https://github.com/login/device/code' `
  -Headers @{ Accept='application/json' } `
  -Body @{ client_id='178c6fc778ccc68e1d6a'; scope='repo workflow read:org' }
# $r.user_code / $r.verification_uri / $r.interval / $r.expires_in（899 秒）
```

`178c6fc778ccc68e1d6a` 是 GitHub CLI 的公开 OAuth app id；长期使用的工具应换成自己的 app id。
把 `user_code` 给人看，让 TA 打开 <https://github.com/login/device> 输入，然后轮询：

```powershell
Invoke-RestMethod -Method Post -Uri 'https://github.com/login/oauth/access_token' `
  -Headers @{ Accept='application/json' } `
  -Body @{ client_id='178c6fc778ccc68e1d6a'; device_code=$r.device_code; `
           grant_type='urn:ietf:params:oauth:grant-type:device_code' }
```

`authorization_pending` → 继续等；`slow_down` → 拉长间隔；`expired_token` → 重新申请（旧码作废，
先把旧轮询停掉）。

### 4. 推送

```sh
git -c http.proxy=http://127.0.0.1:7890 -c credential.helper= \
    push https://x-access-token:$TOKEN@github.com/<user>/mush8-ios.git main:main
```

- `-c credential.helper=` 用来抑制 Windows 凭据管理器弹窗（无界面时它会报
  `failed to execute prompt script (exit code 1)`）。
- token 只出现在**命令行**，不写进 `.git/config`，更不提交。
- `http.proxy` 仅本机网络需要（见 P2）。

### 5. 校验远端一致

```sh
git rev-parse HEAD          # e9970139dd312d7caff0da5c506d8fe9af2f7a9a
git ls-remote <token-url>   # HEAD 与 refs/heads/main 必须打印同一个哈希
```

`/repos/<user>/<repo>` 返回 `visibility=public`、`default_branch=main`；刚建好时 `size` 会是 `0`，
GitHub 还在异步索引，属正常现象。

### 6. 校验渲染结果

```sh
curl -s https://raw.githubusercontent.com/<user>/mush8-ios/main/README.md | wc -c  # 10301
```

再在浏览器里确认：README 正常渲染、语言互链可点、两份声明文件可访问。可选地补描述与 topics：

```powershell
Invoke-RestMethod -Method Patch -Uri 'https://api.github.com/repos/<user>/mush8-ios' -Headers $h `
  -Body (@{ description='学习笔记：…'; topics=@('ios','metal','reverse-engineering') } | ConvertTo-Json -Compress) `
  -ContentType 'application/json'
```

## C. 十一个坑与真正的修法

| # | 现象 | 原因 | 修法 |
|---|---|---|---|
| P1 | `术语 'gh' 不会被识别` | winget 装的目录没进当前 PATH | 用全路径 `C:/Program Files/GitHub CLI/gh.exe`，或重开终端 |
| P2 | `fatal: unable to access …: Could not resolve host: github.com` | Git 走自带 c-ares，不用 .NET 那套 Windows 解析 | 走本地代理：`git -c http.proxy=http://127.0.0.1:7890` |
| P3 | PowerShell 的 REST 正常，`gh` 却报 `error connecting to api.github.com` | 代理只配在 WinINET（`ProxyEnable=1, 127.0.0.1:7890`），`HTTP(S)_PROXY` 为空、WinHTTP 是直连 | 给 gh 导出 `HTTPS_PROXY`/`HTTP_PROXY`；给 git 传 `-c http.proxy` |
| P4 | `error validating token: missing required scope 'read:org'` | 设备流只申请了 `repo`；`gh auth login` 强制校验 `read:org` | 设备流 scope 加上 `read:org`，或干脆绕过 gh，用 REST + token URL 推送 |
| P5 | `refusing to allow an OAuth App to create or update workflow … without 'workflow' scope` | 仓库里有 `.github/workflows/build-ipa.yml` | token 必须带 `workflow`：重新授权 `scope=repo workflow read:org`，或复用已有 `repo, workflow` token（用响应头 `x-oauth-scopes` 先确认） |
| P6 | 约 15 分钟后 `expired_token` | 设备码只有 899 秒 | 重新申请；同时杀掉旧轮询，避免两个循环打架 |
| P7 | `failed to execute prompt script (exit code 1)` / `could not read Username` | 受限 shell 里没有可用的凭据助手 | 单次推送用 `-c credential.helper=` + token URL |
| P8 | `powershell -Command "..."` 里 `$env:...` 莫名变空 | bash 在 PowerShell 看到之前就把 `$var` 展开了 | 写成 `.ps1`，用 `pwsh -NoProfile -File` 执行 |
| P9 | `git status` 里冒出未跟踪的 `CON` | 重定向生成了 Windows 保留设备名 | 不要无脑 `git add -A`；这种名字本来也提交不了 |
| P10 | 推送成功但 API 显示 `size=0 KB` | GitHub 异步索引 | 等约 10 秒再读 `/repos` |
| P11 | 跑了 `gh auth setup-git` 仍未认证 | 它只为「已登录的主机」写凭据助手，而 P4 导致没登录成功 | 改为每次推送自带认证（P7），不依赖全局助手 |

### 两个最不直观的，展开说

**P2/P3 —— 两种不同的「没网」。** 同一个 shell 里 `curl`/`nslookup` 解析不了 `github.com`，但
`[System.Net.Dns]::GetHostAddresses('github.com')` 能拿到 `20.247.166.64`，`Invoke-RestMethod`
也能直连 API。Git for Windows 用自己的 c-ares 解析，于是和 `curl` 一起失败。机器上其实有
WinINET 代理（`127.0.0.1:7890`），.NET 隐式用了它，`git`/`gh` 却不会读。把各自显式指向代理后，
两边立刻通。

**P5 —— scope 是按文件类型卡的，不是按仓库。** 只有 `repo` 的 token 能建仓库、改元数据、推 320 个
文件，但遇到那一个 workflow 文件就会被拦，而且 GitHub 会拒绝**整个 push**（什么都没上去），
报错信息里点名该文件。两条正路：补授权，或换用已有该 scope 的 token —— 先用响应头确认：

```powershell
(Invoke-WebRequest -Uri 'https://api.github.com/user' -Headers @{ Authorization="token $T" }).Headers['x-oauth-scopes']
```

## D. 卫生检查清单

- [x] token 只在 shell 里用过；`git remote -v` 里没有 `token@` 形式的 URL
- [x] `git remote -v` 显示干净的 `https://github.com/<user>/<repo>.git`
- [x] 推送后删除了临时脚本与 token 文件
- [x] `git status --porcelain` 为空；没有 `.nsp`/`keys`/采集产物被跟踪
- [x] 首传直接推 `main`，之后走普通 PR
- [ ] 一旦 token 出现在聊天/日志里，去 <https://github.com/settings/tokens> 吊销并重发
      （「删掉文件」不算处理）

## E. 从零复现发布（精简版）

```powershell
$repo = 'D:\path\mush8-ios'
git -C $repo status --porcelain                     # 空
$h = @{ Authorization = "token $TOKEN"; Accept = 'application/vnd.github+json'; 'User-Agent' = 'notes' }
Invoke-RestMethod -Method Post -Uri 'https://api.github.com/user/repos' -Headers $h `
  -Body (@{ name='mush8-ios'; private=$false } | ConvertTo-Json -Compress) -ContentType 'application/json'
git -c http.proxy=http://127.0.0.1:7890 -c credential.helper= `
    -C $repo push https://x-access-token:$TOKEN@github.com/<user>/mush8-ios.git main:main
git -C $repo rev-parse HEAD
git -C $repo -c http.proxy=http://127.0.0.1:7890 ls-remote https://x-access-token:$TOKEN@github.com/<user>/mush8-ios.git
```

任何一步失败，都到 C 节的表里按报错信息查修法。

---

*仅供学习与研究；见 [`DISCLAIMER.zh.md`](../DISCLAIMER.zh.md)。不分发任何游戏数据。*
