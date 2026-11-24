# KaihongOS（深开鸿 x86 PC）移植记录

日期：2026-11-24。将 msime 移植到深开鸿 x86 PC（bundleName `com.kaihong.khinput`），含五笔 86 输入。

## 设备

- 深开鸿 PC，x86_64，API 23（6.1.0(23)），devicetype "pc"，屏幕 1366x768
- hdc 连接：`192.168.1.47:10178`，shell 为 root（uid=0）
- 已有输入法：com.kaihong.inputmethod（系统「开鸿输入法」）、com.kaihong.msime（BASIC_MODE 旧移植）

## 构建步骤（全部在 macOS 完成）

1. 资源：`MSIME_HARMONY_OMIT_ON_DEMAND=0 bash platforms/harmony/stage-resources.sh target/resources/<hash>`
   - 必须 `OMIT_ON_DEMAND=0` 才会把 msime-wubi.db（136MB）打进 resfile（共 343MB）。
   - rerank 模型需手工放入 `target/rerank-model/`（镜像下载）。
   - macOS bash 3.2 的 nounset 空数组 bug 已在脚本内修复。
2. 原生库：`RUSTUP_TOOLCHAIN=stable bash platforms/harmony/build-native.sh x86_64`
   - 产物在 `target/ohos/libs/x86_64/`，自动 stage 到 `entry/libs/x86_64/`（libmsime_host_api.so 208MB 未 strip，同上游发布流程）。
3. 改包名：
   - `AppScope/app.json5` → bundleName `com.kaihong.khinput`（显示名称保持上游「水杉输入法」不变）
   - `KeyboardExtensionAbility.ets` 的 `BUNDLE_NAME` 常量
   - `build-profile.json5` SDK 版本 → `6.1.0(23)`
   - `entry/src/main/module.json5`：删除 `dataGroupIds` 与 `READ_PASTEBOARD`（debug profile 无 ACL/data-group；代码有运行时回退）
4. HAP 构建：
   ```bash
   export DEVECO_SDK_HOME=/Applications/DevEco-Studio.app/Contents/sdk
   export JAVA_HOME=/Applications/DevEco-Studio.app/Contents/jbr/Contents/Home
   /Applications/DevEco-Studio.app/Contents/tools/hvigor/bin/hvigorw assembleHap \
     --mode module -p product=default -p buildMode=debug --no-daemon
   ```
5. 签名（SDK 自带开源 PKI，全离线）：
   ```bash
   LIB=$DEVECO_SDK_HOME/default/openharmony/toolchains/lib
   java -jar $LIB/hap-sign-tool.jar sign-profile -mode localSign \
     -keyAlias "openharmony application profile debug" -keyPwd 123456 \
     -profileCertFile $LIB/OpenHarmonyProfileDebug.pem \
     -inFile signs/khinput-debug-profile.json -signAlg SHA256withECDSA \
     -keystoreFile $LIB/OpenHarmony.p12 -keystorePwd 123456 -outFile signs/khinput-debug.p7b
   java -jar $LIB/hap-sign-tool.jar sign-app -mode localSign \
     -keyAlias "openharmony application release" -keyPwd 123456 \
     -appCertFile signs/app-debug-chain.pem -profileFile signs/khinput-debug.p7b \
     -inFile entry/build/default/outputs/default/entry-default-unsigned.hap \
     -keystoreFile $LIB/OpenHarmony.p12 -keystorePwd 123456 \
     -outFile dist/khinput-v0.5.0-debug-signed.hap \
     -signAlg SHA256withECDSA -compatibleVersion 23 -signCode 1
   ```
   profile JSON 的 device-ids 填真机 UDID（`hdc shell bm get -u`），bundle-name 必须与包名一致。
6. 安装与启用：
   ```bash
   hdc install -r dist/khinput-v0.5.0-debug-signed.hap   # 453MB
   hdc shell "ime -e com.kaihong.khinput -f"             # FULL_EXPERIENCE_MODE 必须 -f
   hdc shell "ime -s"                                    # 设为当前
   ```
   BASIC_MODE 下有 seccomp SIGSYS 问题（上游 README 已记录），必须完整体验模式。
7. 「设为默认」弹窗切换被拒（errorCode 58）：
   - 现象：设置页横幅点「设为默认」，在系统输入法列表（InputMethodListDialog）里选「水杉输入法」无反应。
   - 根因：深开鸿上弹窗的切换请求以应用自身身份发出（trigger=0），本应用非当前输入法时，框架要求持有
     `ohos.permission.CONNECT_IME_ABILITY`（system_core，stock HarmonyOS 的弹窗切换不走该校验）。
   - 修复：module.json5 声明该权限；签名 profile 的 `apl` 提到 `system_core` 并把它加进 `allowed-acls`，
     重签 profile 与 HAP。验证：横幅点「设为默认」→ 选「水杉输入法」→ `ime -g` 即变 com.kaihong.khinput。

## 五笔 86 切换与验证（已在真机全通过）

方案偏好持久化在设备上（root 可改）：
`/data/app/el2/100/base/com.kaihong.khinput/haps/entry/files/state/preferences.json`
- `"scheme": "wubi"`（默认 quanpin）
- `"wubi_profile": "wubi86"`
改后 `kill -9 <inputMethod pid>` 重启扩展进程即生效（空格键面显示「五笔 86」）。

验证结果（深开鸿设置应用搜索框，物理键盘注入）：
- 一级简码：`q` → 候选 我(trnt) 金(qqqq)…，空格上屏「我」✓
- 全码：`wqiy` →「你」、`vbg` →「好」，连续上屏「你好」✓
- 退格清编码、ESC（先清组字再收面板）、Ctrl+A 全选均正常。

## uinput 按键注入速查（ArkUI KeyCode，字母按字母表排！）

| 键 | 码 | 键 | 码 |
|---|---|---|---|
| 0-9 | 2000-2009 | ESC | 2070 |
| A-Z | 2017-2042 | CTRL_LEFT | 2072 |
| TAB | 2049 | DEL(退格) | 2055 |
| SPACE | 2050 | ENTER | 2054 |

- 字母键码按字母表顺序：A=2017 … Q=2033 … Z=2042（不是 QWERTY 扫描序）。
- 触点：`uinput -T -c x y`；按键：`uinput -K -d <码> -u <码>`（-d/-u 必须同命令成对）。
- 截屏：`snapshot_display -f /data/local/tmp/x.jpeg`（必须 /data/local/tmp 下），`hdc file recv` 取回。

## PC 形态修复（去虚拟键盘）

深开鸿 PC 的 `deviceInfo.deviceType` 是 `"pc"`，而上游 `KeyboardFormFactorPolicy.isDesktop` 只认 `"2in1"`，
导致误走触屏分支、弹出整块软键盘（[225,104,1366x271]）严重遮挡屏幕。

修复（`entry/src/main/ets/keyboard/KeyboardFormFactorPolicy.ts`）：

```ts
static isDesktop(deviceType: string | null | undefined): boolean {
  return deviceType === "2in1" || deviceType === "pc";
}
```

修复后日志 `panel ready: pc, candidate window, with toolbar`，行为与 2in1/Windows 一致：
- 无软键盘；组字时在光标旁显示 280x232 候选窗，组字结束自动隐藏
- 右下角 303x44 浮动状态条（不想可用 `floating_toolbar.enabled=false` 关闭）
- 硬件键盘输入路径不变（五笔 86 验证仍通过：q+空格→我）

## 已知事项

- sherpa_onnx.har（语音）经镜像下载，HAP 内含双架构（arm64-v8a + x86_64）无害。
- 桌面仍显示 com.kaihong.msime 为「水杉输入法」——那是旧移植包，与本包无关。
