# 🐧 lazywslc

A [**lazydocker-inspired**](https://github.com/jesseduffield/lazydocker) TUI dashboard for managing WSL Linux containers via `wslc.exe`.

![Featured Image](./docs/hero_image.png)

## Install

For now [install the latest from the GitHub releases page](https://github.com/craigloewen-msft/lazywslc/releases). 

Winget support exists but its version is older and won't work well. [PR is in progress](https://github.com/microsoft/winget-pkgs/pull/441174). 

```powershell
winget install crloewen.lazywslc
```

Start the terminal UI

```powershell
lazywslc
```

## Features

- Three-panel layout for Containers, Images, and Volumes
- Live CPU/memory sparkline graphs
- Combined logs and stats view
- Mouse and keyboard support
- Context actions (start/stop/kill/remove/prune)
- Interactive container shell (`E`), using the container user's configured shell
- Optional `wslc-remote` integration for safe NFS-backed container start/recovery
- Auto-refresh

## wslc-remote integration

When [wslc-remote](https://github.com/craigloewen-msft/WSLc-remote) is installed in your default WSL distribution, LazyWSLc automatically routes **container start** and **volume remove** through the Linux wrapper. This lets stopped containers with NFS-backed `wslc-remote` volumes recover correctly after a WSL or WSLC runtime shutdown.

All normal read/list/log/stop/kill operations continue to use native `wslc.exe`; only lifecycle-sensitive operations that need NFS recovery/teardown are routed through the wrapper.

Environment variables:

- `LAZYWSLC_REMOTE=0` disables wslc-remote detection.
- `LAZYWSLC_REMOTE_DISTRO=<name>` selects a WSL distro explicitly instead of using the default distro.

If the wrapper is not available, LazyWSLc falls back to native `wslc.exe start`. If the wrapper is detected but recovery fails, LazyWSLc surfaces that error instead of bypassing recovery.

Running containers also expose **Exec Shell** through the action menu (or the `E` hotkey), following lazydocker's terminal handoff model.
## Support 

Although I do work on the WSL team, this is being submitted as a community project from me personally (Craig Loewen) not as a representative of the WSL team. So this project will have community level support. Please treat it as you would any other open source community project.

## Contribute

Feel free to fork and open PRs!

## License

MIT
