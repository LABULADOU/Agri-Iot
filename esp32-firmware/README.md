# 农业物联网 ESP32 固件烧录指南

## 在 Android 手机上烧录 ESP32

### 1. 安装 Termux

1. 从 F-Droid 或 GitHub 安装 Termux
   - https://f-droid.org/packages/com.termux/
   - 或 https://github.com/termux/termux-app/releases

2. 打开 Termux，执行初始化：
   ```bash
   pkg update && pkg upgrade -y
   ```

### 2. 安装依赖

```bash
# 安装 Python 和 esptool
pkg install -y python git make

# 安装 esptool（烧录工具）
pip install esptool

# 安装 PlatformIO（可选，用于编译固件）
pip install platformio
```

### 3. 连接 ESP32

1. 准备 USB OTG 转接头（Type-C 转 USB-A）
2. 将 ESP32 通过 USB 线连接到 OTG
3. 在 Termux 中检查串口：
   ```bash
   ls /dev/tty* | grep -E "USB|ACM"
   ```
   应该看到类似 `/dev/ttyUSB0` 或 `/dev/ttyACM0`

### 4. 修改固件配置

编辑 `esp32-firmware/main.ino`，修改以下配置：

```cpp
// WiFi 配置
const char* WIFI_SSID = "你的WiFi名称";
const char* WIFI_PASSWORD = "你的WiFi密码";

// MQTT 服务器地址（替换为你的后端 IP）
const char* MQTT_SERVER = "192.168.1.100";

// 设备唯一标识
const char* NODE_ID = "esp32-node-001";
```

### 5. 编译固件

#### 方式 A：GitHub Actions 云端编译（推荐，适用于 Termux/Android）

项目已配置 GitHub Actions 自动编译工作流，无需在手机上安装编译工具链：

1. 将代码推送到 GitHub 仓库
2. 在 GitHub 项目的 **Actions** 页面查看构建进度
3. 构建完成后，在 **Artifacts** 中下载 `esp32-firmware-bin.zip`
4. 在 Termux 中使用烧录脚本：
   ```bash
   bash flash_from_ci.sh <下载链接>
   ```

#### 方式 B：使用 PlatformIO（需要 x86_64 Linux 环境）

```bash
cd esp32-firmware
pio run -e esp32dev
```

编译后的固件位于：`.pio/build/esp32dev/firmware.bin`

> **注意**：PlatformIO 在 Termux/Android 的 F2FS 文件系统上可能无法正常工作（工具链文件名包含 `++` 字符，F2FS 不支持）。建议使用方式 A。

### 6. 烧录固件

```bash
cd esp32-firmware

# 使用 esptool 烧录
esptool.py --port /dev/ttyUSB0 --baud 921600 write_flash 0x0 firmware.bin
```

或使用自动脚本：
```bash
# 方式 1：使用预编译固件（推荐）
bash flash_from_ci.sh <固件下载链接>

# 方式 2：本地编译后烧录
bash flash.sh
# 选择 3 (编译并烧录)
```

### 7. 验证烧录

烧录后 ESP32 会自动重启，查看串口输出：

```bash
python -m serial.tools.miniterm /dev/ttyUSB0 115200
```

应该看到：
```
=== 农业物联网 ESP32 固件 v1.0 ===
连接 WiFi: YOUR_WIFI_SSID
WiFi 已连接! IP: 192.168.1.xxx
连接 MQTT Broker... 已连接
已订阅: agri/node/esp32-node-001/command/#
温度: 25.3℃ | 湿度: 60.0% | 土壤湿度: 45% | 光照: 3200 lux | 上报: 成功
```

### 8. 在后端添加设备

在 Web 界面中添加对应的传感器和执行器：

1. 访问 `http://你的服务器IP:3000`
2. 进入"设备管理"
3. 添加设备，节点 ID 填写固件中的 `NODE_ID`

---

## 常见问题

### Q: 找不到串口设备
- 确保 OTG 连接正确
- ESP32 使用 CH340/CP2102 芯片的需要驱动
- 尝试 `lsusb` 查看 USB 设备列表

### Q: 烧录失败
- 确保 ESP32 进入烧录模式（按住 BOOT 按钮后插入 USB）
- 降低波特率：`--baud 115200`
- 检查 USB 线是否支持数据传输

### Q: MQTT 连接失败
- 确认 `MQTT_SERVER` 填写的是服务器的局域网 IP
- 确保手机和 ESP32 连接同一个 WiFi
- 确认后端 MQTT Broker 端口 1883 已开放

### Q: PlatformIO 编译失败（Invalid argument / 找不到 g++）
- 这是 Android F2FS 文件系统的限制，不支持文件名中包含 `++` 字符
- ESP32 工具链的核心编译器 `xtensa-esp32-elf-g++` 无法解压
- **解决方案**：使用 GitHub Actions 云端编译，下载预编译固件后烧录
