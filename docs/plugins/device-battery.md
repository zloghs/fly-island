# 设备电量 / Device Battery

`device-battery` 是 `feat/plugin-pages` 上的离线 Windows 插件。启用后在岛屿顶栏出现电池页，显示设备名、剩余百分比、连接方式、充电状态和上次成功读取时间。使用 API v1 的 `panel`，不占用 AI 额度环，不自建窗口，不修改宿主命令。

## 协议来源与兼容范围

思路参考 [CoraxEldridge/2.4G-Battery](https://github.com/CoraxEldridge/2.4G-Battery)，协议核对了 [迈从官方驱动](https://www.mchose.com.cn/#/connectDevice) 和 [雷柏官方驱动](https://hub1.rapoo.cn/home)（2026-11-18）。本插件独立实现读取、解析、状态和界面快照，没有复制这些项目的代码、资源或网页脚本。参考项目没有通用的 2.4G 电量协议，也不能用一个字节偏移支持所有品牌。

| 设备 | 识别与读取 | 解析（从 0 开始；编号报告包含 Report ID，无编号报告不包含 Windows 的补位 0） |
| --- | --- | --- |
| 迈从 G87 2.4G | `41E4:2001`，usage page `FF04`，8 字节 Feature Report `06` | `report[3]` 为百分比；`report[2]` 为状态：0 使用中、1 充电中、2 且电量 100% 为充满，其他为未知 |
| 迈从 K99 V3 2.4G / USB | 接收器 `3837:3033` 且产品名精确为 `MCHOSE K99 V3 2.4G`，有线 `3837:2020`；usage `0001:0000`，描述符必须为无编号的 64 字节 IN / OUT | 官方 GLW `getFuncConfig` 命令 `05`，只读 profile 0 的偏移 `32`、长度 `1`；响应数据 `report[8] & 0x7F` 为电量，`report[8] & 0x80` 为充电标志 |
| 雷柏 VT 二代系列，官方 `VT_nrf54L` 协议 | VID `24AE`，usage page `FF00` / usage `0002`，只对 `catalog.rs` 中已核对的鼠标 PID 与接收器 PID 监听 Input Report `07` | `report[1] & 0x0F`：0 为 2.4G、1 为蓝牙、2 为 USB；`report[7]`：1 使用中、2 充电中、0 无效；`report[8]` 为百分比 |

雷柏型号表保存官方目录中 61 个该协议的鼠标型号，包括 VT0 / VT1 / VT2 / VT3 / VT3S / VT7 / VT9、MAX / Air 与部分 V2 / MASTER 型号，完整 PID 对照以 [catalog.rs](../../plugins/device-battery/src/catalog.rs) 为准。你的 VT0：有线 `24AE:4610`、2.4G 接收器 `24AE:1410`。名字相似的旧型号不一定使用相同协议。雷柏底座、其他协议、迈从其他键盘和罗技 HID++ 尚未适配；诊断模式只枚举未适配的 vendor collection 和迈从 Generic Desktop / Undefined 命令接口，不打开它们。

K99 V3 按[官方 GLW SDK](https://www.mchose.com.cn/cizhou/CZ_SHARED_DATA/main.002bbdb0f17b5f7135ee.js)的 `getFuncConfig` / `Le` 位域解析实现；源码日期同上。查询载荷为 `55 05 00 21 01 20 00 00` 加零至 64 字节，Windows/hidapi 输出前补无编号 Report ID `00`，输入时 hidapi 会去掉该补位。校验响应长度、`AA 05`、数据长度与偏移后才读取电量。没有擅自把 `getInfo` 命令 `03` 的固件版本当成电量，也不推断充满状态。`3837:3033` 被多款迈从接收器共用，必须同时核对产品名、collection 和报告尺寸，不能仅凭 VID:PID 发查询；boot mouse 和升级接口均拒绝。K99 V3 的实现与 G87 的 Feature Report 通道相互独立。

雷柏按官方驱动的输入报告方式被动监听。插入接收器后可能需要移动鼠标或切换充电状态才会上报；「重新扫描」重新枚举并重试打开设备，不向鼠标发送唤醒或配置指令，也不保证立刻得到新电量。USB 模式在已支持的雷柏和 K99 V3 接口上识别；不扫描系统蓝牙电量。

**0.1.1 修复**：Windows 将雷柏的 HID 顶层 collection 拆成独立设备路径。原先误开 `FF00:000E` 配置通道，导致 VT0 被发现后始终「等待电量报告」；现改为只监听 `FF00:0002` 状态通道，不把配置通道显示成重复设备。官方驱动的设备校验同时接受 `000E` / `0002`，WebHID 设备不能直接等同于单个 Windows HID 路径。状态接口和 16 / 19 字节报告另与 [Windows 实机抓包记录](https://github.com/krisgfx9/Wireless-Device-Battery-Monitor/blob/main/docs/rapoo.md)交叉核对；回归测试使用其中的状态样本，未复制实现代码，也未加入格式未确认的主动查询。

以上是源码协议适配范围，尚未在本次开发中用真实设备验证。报告里的百分比不代表实测精度，充电状态须与对应固件实际行为对照。

## 设置与失败行为

- 默认启用两家读取，扫描 / 键盘读取间隔 30 秒，可改 5 / 60 秒。雷柏与 K99 V3 句柄持续保留，每秒非阻塞接收少量报告；K99 V3 每次只发固定的电量读取请求，等待响应上限 1.5 秒，超时保留旧值并在下次扫描重试。不打开普通键盘或鼠标输入 collection。
- 你的两台设备可筛选为 `3837:3033,24AE:1410`（大小写均可）；空白显示全部已适配设备。K99 V3 有线测试时需把 `3837:2020` 也加进去。格式错误时停止硬件访问并提示修正。相同型号的多个接收器以设备路径的哈希区分，不把路径或序列号发给宿主。
- 电量不高于阈值（默认 20%，可设 1–50%）且近期读数未显示充电 / 充满时，行变为 warning 并点亮页按钮，不弹系统通知。真实有效的 0% 可以显示，读取失败不会制造 0%。
- 超过两分钟未收到有效报告，标为「等待上报／可能休眠」，保留旧读数并显示样本年龄；接口仍在并不等于无线设备一定在线。收到无效状态或 I/O 失败时同样保留旧读数，取消低电量提示。
- 拔掉接收器后在下次扫描标注断开；最近样本在读取后十分钟内可作为旧值保留。最多 16 行，当前连接设备优先。重新连接后必须再次收到有效报告才能恢复有效状态。
- 诊断模式附加 VID:PID 和最近的原始电量报告（含无效报告），雷柏行还显示所监听的 `usage FF00:0002 · RID 07`；原始报告为 `—` 表示尚未收到数据。这些数据只在内存和宿主面板里显示，不写日志或文件。关闭 / 启动会清空样本，偏好由宿主插件 settings 保存。
- 读取在线程中进行，不阻塞 stdio 握手、刷新动作响应或退出。停用 / 卸载 / 退出时关闭进程，系统释放 HID 句柄；异常缓慢的内核 Feature 读取不会拖住宿主退出。

只声明 `panel` 权限，无联网、文件读取、通知或桌面窗口权限。现有宿主没有单独的 HID 权限枚举；权限是行为声明，不是操作系统沙箱。依赖 hidapi 2.6.7 的 `windows-native` 后端，不附带 C HIDAPI DLL，也不写设备配置或固件。

## 自行构建和安装

按本次要求，未执行宿主构建、release 构建或插件打包，未生成安装包。需要使用此分支的宿主（支持 panel 与 number 设置）；正式版本若关闭插件发布开关则没有插件入口。

在仓库根目录运行以下命令构建插件本地分发包：

```powershell
powershell -NoProfile -File scripts/release/build-plugins.ps1 -Ids device-battery -OutputDirectory artifacts/plugins/device-battery-preview
```

宿主设置 → 插件 → 开发者，启用开发模式，将 `artifacts/plugins/device-battery-preview/plugins-index.json` 的绝对路径填为插件索引，保存并刷新，安装「设备电量」并启用。现有脚本自动发现清单，无需改发布脚本。插件是单独的下载包；仅构建宿主安装包不会把插件 exe 编入宿主。

人工验收建议：

1. 先用官方驱动确认设备型号、电量和充电状态，记下 VID:PID，再关闭驱动启用插件。
2. 插着 2.4G 接收器，轻触键盘 / 移动鼠标，对照百分比；开启诊断记录 VID:PID 和报告，检查上述偏移及状态。
3. 插拔充电线，确认充电状态；雷柏充电状态 2 只表示充电，不擅自推断「已充满」。K99 V3 同时验证接收器和 `3837:2020` 有线接口；G87 有线直连不在本次适配范围。
4. 让设备休眠超过两分钟、拔插接收器、同时打开官方驱动，检查旧值提示、重新扫描和恢复；不把「接收器存在」误当成设备在线。
5. 用不存在的筛选、错误筛选、同型号双接收器、中英文和窄岛宽检查空状态、独立识别、换行与滚动。停用插件后确认电池页消失且进程退出。

## 自动验证

已运行插件专用检查，不接触真实 HID 设备：

```powershell
cargo test --manifest-path plugins/device-battery/Cargo.toml --target-dir plugins/target --locked --offline --config build.incremental=false
cargo clippy --manifest-path plugins/device-battery/Cargo.toml --all-targets --target-dir plugins/target --locked --offline --config build.incremental=false -- -D warnings
node scripts/check-file-size.mjs
```

14 项单元测试和 1 项真实进程协议测试通过；Clippy 0 告警，文件规模检查通过。覆盖百分比与充电解析、Report ID 偏移、无效 / 截断报告、K99 V3 固定请求与电量位域、共用 PID 的产品名约束与描述符尺寸、VT0 接收器 / 有线状态 collection 与配置通道排除、雷柏 16 / 19 字节实测报告样本、过滤失败关闭、旧读数与低电量状态、真实宿主 manifest / publish schema，以及 initialize → publish → rescan / error → shutdown / EOF。进程测试关闭两家 provider，确保没有硬件访问。以上检查不替代真实设备和岛屿页面验收。

## English

Device Battery is an optional offline Windows process using the island's native battery panel. It supports MCHOSE K99 V3 (`3837:3033`, exact receiver product name required; wired `3837:2020`), the G87 receiver (`41E4:2001`, feature report `06`), and the Rapoo VT Gen 2 catalog including VT0 (`24AE:1410` wireless / `24AE:4610` wired, input report `07`). Other protocols and charging bases are unsupported. K99 V3 uses the official GLW read-config command for the packed battery/charging byte at offset 32; it requires an unnumbered 64-byte IN/OUT command collection and never changes device configuration.

Rapoo monitoring is passive on Windows collection `FF00:0002`, input report `07`. Version 0.1.1 fixes opening the separate `FF00:000E` configuration collection, which could leave VT0 waiting indefinitely. Regression coverage includes captured 16/19-byte receiver and cable status frames; this is not physical VT0 validation. Diagnostics show the selected usage/report ID and raw data (`—` means nothing received). Move/wake the mouse or change its charging state to obtain a report. Rescan retries discovery and access; it does not send device configuration or wake commands. Readings become stale after two minutes, and failures retain the previous percentage with an explicit age/status. Fresh low readings highlight the row and page button; charging readings do not trigger attention. No battery data is persisted or sent over the network.

Build and install with the command above and the host's developer index. The host installer does not bundle this plugin. Only plugin tests, host-schema validation and lint checks were run; no installer/release package or real-device acceptance test was performed. Enable diagnostics to compare raw reports with the official driver during manual testing.

## 本次规范核对 / Guide review

0.1.2 将接收检查与停用等待降到每秒一次，默认仍每 30 秒枚举；保留 K99 V3 固定读取和 VT0 状态 collection 修复。页面参与 Ctrl + 滚轮切换，诊断文本允许换行。正式发布门禁保持关闭，本地测试请按 [quickstart](quickstart.md) 临时启用 `plugins`；仅构建宿主不包含独立插件包。完整核对见 [compliance-review](compliance-review.md)。

Version 0.1.2 reduces receive checks to once per second and keeps 30-second discovery, the fixed K99 V3 query and the VT0 status collection. Native pages join Ctrl + wheel navigation and wrap diagnostic text. The production gate stays closed; temporarily enable it locally via the quickstart. No installer, release build or plugin package is produced by this task.
