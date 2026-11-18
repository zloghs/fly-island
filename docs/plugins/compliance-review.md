# 插件规范核对 / Plugin guide review

基线为 `origin/main` 的 `abd7cb2`。本次只核对 `feat/plugin-pages` 新增的护眼和设备电量插件；不改其他存量适配器。

新版开发规范已明确允许独立窗口。护眼 0.2.0 改为宿主按需创建的 Tauri 多屏休息窗口、一次推迟、可选跳过与宿主窗口层级保护；移除旧版禁窗规则及用户豁免的过时说明。

| 规范 | 核对与调整 |
| --- | --- |
| 融入源码、接入点集中 | React 原生页面位于 `src/features/plugins/PluginPanel.tsx`；顶栏 `IslandHeader`、内容 `IslandPages`、功能 `useIslandFeatures`、尺寸 `expandedHeight` 接入；设置复用 `PluginSettingRow` / `PluginNumberSettingRow`。新主分支的 `wheelPages` 也纳入插件页。采集器不向宿主提供 HTML/CSS。 |
| 默认不安装、可关闭 | 恢复主分支的 `release-features.json`，不提交开启的发布门禁。没有安装/启用时不显示页面或启动进程；停用和卸载退出。按 quickstart 仅在本地临时开启门禁测试。 |
| 原生组件、双语、窄窗口 | 使用宿主主题 token、lucide 图标和设置组件。长说明、诊断和错误换行，按钮行可换行；修复长说明挤压诊断区域的问题，内容超高时整体滚动。清单和控件均中英文；动态状态使用插件显式语言设置，现协议不提供自动跟随宿主 locale。休息窗口也复用宿主主题、SettingsLinkButton、lucide 和双语字典，不再使用固定配色的 GDI 按钮。 |
| 契约与数据兼容 | 保持插件 id、命令、事件、Tauri 窗口标签与既有存储 key。护眼保留旧秒数/小数分钟向上取整迁移；偏好用既有 `plugins-state.json`，采样和计时只留内存。 |
| 最小权限、隐私 | 无联网、遥测或无关文件读取。护眼声明 panel/desktop-windows，只读空闲和全屏、由宿主创建休息窗口并约束其自身层级；电量只打开白名单 HID collection，K99 固定查询不改配置/固件。诊断不发送路径和序列号。 |
| 性能与退出 | 护眼后台与休息窗口定时器改为每秒一次，层级纠正仍走同步窗口事件；电量接收每秒一次，默认 30 秒枚举。句柄、报告、缓存、队列有界；shutdown/EOF 有协议测试，遮罩真实退出仍待桌面验收。 |
| 新版插件市场 | 保留新的方形卡片、内置插件安装状态与来源开关。主分支已提交 `features/plugins/native/catalog`，删除上一轮的临时字面量类型，重新复用原生目录类型；外部 panel 插件仍按现有本地目录/索引机制安装，不冒充内置卡片。 |

## 验证边界

按用户要求不运行 `pnpm build`、Tauri build、插件 build 或发布/打包脚本；只执行类型/静态检查、单元/进程协议测试和浏览器回归。具体结果随最终回复报告。宿主安装流程测试依赖预先构建的 example-hello 和打包索引，当前缺少有效前置产物时不得伪称通过。

真实 K99 V3 / VT0、电量与充电准确性、休眠恢复、多屏/DPI/遮罩按钮、深浅色/两种玻璃/窄窗口需由用户自行构建验收。CPU、常驻/峰值内存未实测。未创建 PR；上游要求的真实岛屿录屏、作者、公开联系方式与人工自测记录需要作者在提交 PR 时提供，不伪造信息。

## English

Reviewed the two plugins against main at `abd7cb2`, whose guide now permits separate windows. Eye Care 0.2.0 uses host-created Tauri webviews with shared theme tokens, controls and localization; snooze, optional skipping and host layering remain. Native panel/settings integration reuses host theme tokens and controls; long notes can scroll without collapsing diagnostic rows. The new square-card market and built-in plugin installation state remain intact. Since main now provides the native catalog, both branches reuse its shared types instead of the former temporary literal keys. External panel plugins continue to use their existing optional installation path. Release flags, contracts, legacy duration migration and bounded polling remain unchanged.

Only static, unit, protocol and browser checks are performed, with no build/package commands. Physical devices, desktop visual acceptance, multi-monitor behavior and measured resource budgets remain for manual verification. Missing prebuilt host-test fixtures are reported explicitly. No PR is created; real recordings, author/contact details and manual test results must be supplied by the author before an upstream PR.
