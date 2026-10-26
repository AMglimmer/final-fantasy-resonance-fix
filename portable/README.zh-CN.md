# FF Resonance 正式版超宽屏补丁 0.2.1-retail.1 · Windows x64 便携版

基于 PolarWizard 原项目适配 Steam 正式版 **build 24250053**。此包不需要
Rust、Python、UE4SS 或编译环境，使用 Windows 自带 PowerShell 完成复制和恢复。

## 安装

1. 将 ZIP 解压到游戏目录以外、可以写入的文件夹。
2. 退出 FF Resonance，双击 `Install.cmd`。
3. 输入 Steam 正式版根目录，或 `FFRS\Binaries\Win64` 路径。
4. 正常从 Steam 启动游戏，并在游戏里选择原生分辨率。已测试 3440×1440。

脚本会验证游戏 EXE 和补丁哈希，备份会覆盖的文件，只安装 Loader、ASI 和 TOML。
原有 TOML 会保留。Demo 和未验证的游戏版本不会安装；遇到其他 Loader 也不会直接覆盖。
普通可写 Steam 库一般不需要管理员权限。

默认配置：场景全宽、保持垂直视野的 Hor+ 逻辑、16:9 居中 HUD、保留原项目
16:9 电影逻辑；后处理保持游戏设置，字体替换关闭。3440×1440 时 HUD 位于中央
2560×1440 区域。

### 从 0.2.0-retail.1 升级

退出游戏后，使用新版文件夹中的 `Install.cmd` 安装到同一正式版目录即可。
旧 ASI 会备份，现有 TOML 保留。升级后保留新版便携文件夹并使用新版管理脚本。
卸载新版会恢复升级前的文件，其中可能包含旧补丁。

## 禁用和卸载

退出游戏后，在同一个解压文件夹双击：

* `Disable.cmd`：禁用补丁。
* `Enable.cmd`：重新启用。
* `Uninstall.cmd`：按安装记录恢复原文件；修改过的配置另存备份。

请保留整个便携文件夹。安装记录 `.ffrs-portable-state.json` 和 `.ffrs-backups`
用于恢复，不要提前删除。快捷脚本只管理由该文件夹记录的安装。

如需手动安装，先备份目标文件，再将 `payload` 内的内容复制到游戏
`FFRS\Binaries\Win64`。已有其他 ASI Loader 时，只按那个 Loader 的说明放入
`payload/scripts` 中的两个文件，不要覆盖它的代理 DLL。

手动禁用：游戏退出后，把 `scripts/final_fantasy_resonance_fix.asi` 改名为
`final_fantasy_resonance_fix.asi.disabled`。手动安装需自行恢复备份。

配置和日志均在游戏 `FFRS\Binaries\Win64\scripts` 下。

## 已验证范围

采用本机已测试的同一份运行时代码，公开二进制重新构建并移除本机构建路径。
用户已确认场景内及世界大地图生效；运行日志确认 SDK 校验和 HUD/电影回调执行。
本次正式版修复了对话／战斗转场使用已被回收的临时渲染目标而闪退的问题，
临时目标改为每次截图单独创建，绘制结束或失败时释放 GPU 资源。
16 项自动测试、实际转场源码的生命周期回归与 6 组便携安装测试通过。
安装修复后，约 64 分钟监控未捕获新的退出或崩溃，用户随后反馈实际游玩稳定。
CG、全部 UI 和相同相机下的 Hor+ 图片对照尚未穷尽验证；正式发布不代表所有场景均已测完。

独立实验性的后台静音工具不包含在本包内。

游戏更新后需要重新验证。补丁不修改游戏原始 EXE、Pak、存档或安全机制。

原项目 PolarWizard、Ultimate ASI Loader / ThirteenAG 的署名及依赖许可见
`LICENSE`、`LICENSES`、`LICENSE-UAL.txt`。
