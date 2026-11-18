# 岛屿消失：收起歌词递归更新

日期：2026-11-18。

## 异常现场

安装版进程存活，主窗口可见、未最小化、位于屏幕顶部，分层窗口 alpha 为 255；`WM_NULL` 消息在 700ms 内响应，GDI / USER 资源分别为 30 / 70。没有对应的应用崩溃事件。现场截图中岛屿区域为空白。

本机 `.git/island-renderer-failed.dmp` 中提取到实际错误 `Minified React error #185`，即 Maximum update depth exceeded；组件帧包含收起歌词 `HH`（对应 `CollapsedLyric`）、消息摘要与岛屿外壳。内存中的已安装脚本确认该版本的 `useOverflow` 在每次布局副作用中读取变换后的 `getBoundingClientRect`，立即设置 overflow 状态。

托盘 Settings 并非独立的前端恢复入口：`settings_window::request` 先显示主窗口，再向主页面发事件，由主页面决定如何打开设置。React 根卸载后，该入口也无法响应，所以“托盘设置没反应”不能单独证明原生线程卡死。

转储仅保存在本机忽略目录，不提交或上传，可能包含用户页面数据。原生进程转储为 `.git/island-native-failed.dmp`，窗口现场为 `.git/island-windows-failed.json`，截图为 `.git/island-failed.png`。

## 复现与修复

回归页对屏幕坐标读数注入 ±0.01px 的亚像素抖动，旧实现同步反复更新，复现 Maximum update depth exceeded。错误输出保存在本机 `.git/island-loop-before.log`。这证明该测量路径能够导致现场观察到的同类故障，而不是仅凭猜测修改样式。

修复使用同一坐标系的 `scrollWidth` / `clientWidth`，不再用变换后的屏幕宽度反推比例。渲染和 ResizeObserver 只安排下一次 requestAnimationFrame，合并测量，禁止布局提交阶段递归设置状态；观察原文与附加歌词各自的内容节点，卸载 / 替换节点时取消回调。

相同抖动回归修复后通过，保留长句滚动、双行翻译和迟到歌词的原有检查。没有改变用户设置、存储格式或窗口契约。

## 验证

- 修复前同一回归页触发 Maximum update depth exceeded，修复后 23 项检查通过。
- `pnpm check`：416 项测试通过；`pnpm tauri build --no-bundle`：通过，包含前端生产构建。
- 全量界面回归 59 页中 58 页通过。唯一失败是已有 `release-features.html`：当前合并分支已启用插件，页面仍要求两个发布开关关闭，与此次测量逻辑无关，未改动发布配置。
