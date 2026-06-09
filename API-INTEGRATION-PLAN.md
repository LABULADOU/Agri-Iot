# 第三方 API 集成评估

> 对 Agri-Iot 项目可用的公开 API 调研，按优先级分类，供复查筛选。

---

## 目录

1. [已集成的 API](#1-已集成的-api)
2. [作物病害检测（高优先级）](#2-作物病害检测高优先级)
3. [植物/作物数据库（高优先级）](#3-植物作物数据库高优先级)
4. [农业气象数据 API（高优先级）](#4-农业气象数据-api高优先级)
5. [卫星遥感与植被指数（中优先级）](#5-卫星遥感与植被指数中优先级)
6. [农产品行情（低优先级）](#6-农产品行情低优先级)
7. [本地可部署的开源模型（低优先级）](#7-本地可部署的开源模型低优先级)
8. [汇总对比表](#8-汇总对比表)

---

## 1. 已集成的 API

| API | 终端 | 接入方式 |
|-----|------|----------|
| **和风天气 (QWeather)** | `WEATHER_API_KEY` .env 配置 | `agri-server/src/weather.rs` 透明代理，前端 TopBar 展示 |

**已实现的功能：**
- `GET /api/v1/weather/now` — 实时天气（温度、湿度、风向风力、天气现象）
- `GET /api/v1/weather/3d` — 3 天预报
- `GET /api/v1/weather/24h` — 24 小时逐小时预报
- `GET /api/v1/weather/minutely` — 分钟级降水估测（需付费订阅）
- `GET /api/v1/weather/warning` — 灾害预警（需付费订阅）
- `GET /api/v1/weather/air` — 空气质量
- `GET /api/v1/weather/indices` — 生活指数

**限制：** 免费订阅（开发者版）不支持 minutely 和 warning 两个端点。

---

## 2. 作物病害检测（高优先级）

### 2.1 crop.health (Kindwise)

| 项目 | 内容 |
|------|------|
| **网址** | https://crop.kindwise.com |
| **功能** | 上传作物照片，AI 识别病害和虫害，返回诊断结果、置信度、防治建议 |
| **免费额度** | 100 次识别 / 月 |
| **付费** | ~$0.05/次（超过免费额度） |
| **文档** | https://crop.kindwise.com/api/v1/openapi.yaml |
| **SDK** | Python SDK: https://github.com/flowerchecker/kindwise-api-client |

**请求示例：**
```
POST https://crop.kindwise.com/api/v1/identification
Headers: Api-Key: xxx
Body: {
  images: ["base64..."],
  latitude: 39.9,
  longitude: 116.4
}
```

**响应：**
```json
{
  "result": {
    "is_healthy": false,
    "disease": {
      "name": "苹果黑星病",
      "probability": 0.94,
      "treatment": "建议使用...",
      "suggestions": [...]
    }
  }
}
```

**与本项目的集成方式：**
1. `agri-server` 增加 `POST /api/v1/ai/detect-disease` 代理端点
2. 请求转发到 crop.kindwise.com，返回结果缓存到本地
3. 前端增加拍照/上传入口（可直接利用移动端相机）
4. 识别结果写入 Obsidian 02-Cases/ 案例库

**同系列产品（同一家公司）：**
| 产品 | 功能 | 适用场景 |
|------|------|----------|
| plant.id | 植物品种识别 | 杂草/作物分类 |
| plant.health | 观赏植物病害诊断 | 温室花卉 |
| **crop.health** | **大田作物病害/虫害** | **本项目首选** |
| insect.id | 昆虫识别 | 益害虫区分 |

---

## 3. 植物/作物数据库（高优先级）

### 3.1 Flora API

| 项目 | 内容 |
|------|------|
| **网址** | https://floraapi.com |
| **功能** | 29,000+ 北美原生植物数据库：分类、分布（县级精度）、生长条件、毒性、花期 |
| **免费额度** | 1,000 次/月 |
| **付费** | $49/月起 |

**与本项目的集成方式：**
- 增强 Obsidian 00-Crops/ 知识库的自动填充
- 查询作物品种的生长条件（温度范围、水分需求、光照），输入到 AI 评估系统
- 配合区域定位推荐适宜作物

**注意：** 目前仅覆盖美国/北美地区植物。中国作物需另寻来源。

### 3.2 Trefle API

| 项目 | 内容 |
|------|------|
| **网址** | https://trefle.io |
| **功能** | 全球植物数据库，400,000+ 品种 |
| **免费额度** | 曾免费，当前状态需确认 |
| **备注** | 曾被收购，可用性需验证 |

### 3.3 APIFarmer Plant Database API

| 项目 | 内容 |
|------|------|
| **网址** | https://apifarmer.com/plant-database-api/ |
| **功能** | 植物分类学、生长条件、繁殖数据 |
| **免费额度** | 无免费 tier |
| **适用性** | 付费 API，优先级不高 |

---

## 4. 农业气象数据 API（高优先级）

### 4.1 Weatherbit AgWeather API

| 项目 | 内容 |
|------|------|
| **网址** | https://www.weatherbit.io/api/agweather-api |
| **功能** | 全球 0.25° 格点农业气象数据：土壤温度、土壤湿度、蒸散量(ET)、太阳辐射、降水 |
| **免费额度** | 有免费 tier（每日请求数有限） |
| **数据源** | NASA GLDAS + ERA5 再分析 |

**关键参数：**
- Skin/Surface Temperature
- Soil Temperature（多层）
- Soil Moisture（体积含水量）
- Evapotranspiration
- Shortwave/Longwave Solar Radiation
- Specific Humidity
- Precipitation

**与本项目的集成方式：**
- 作为 ESP32 土壤传感器的**外部数据补充/校验**
- 提供历史数据回溯（十年级），用于 AI 模型训练
- 蒸散量(ET)数据可辅助灌溉决策（与 EC 值联动）

### 4.2 和风天气（已有）

当前已在使用的天气 API，功能清单见[第 1 节](#1-已集成的-api)。

---

## 5. 卫星遥感与植被指数（中优先级）

### 5.1 NASA LANCE / MODIS NDVI

| 项目 | 内容 |
|------|------|
| **网址** | https://earthdata.nasa.gov |
| **功能** | 近实时 NDVI（归一化植被指数），250m 分辨率 |
| **费用** | 完全免费 |
| **数据格式** | GeoTIFF, JSON API |

### 5.2 Agromonitoring API

| 项目 | 内容 |
|------|------|
| **网址** | https://agromonitoring.com/api |
| **功能** | 场级卫星影像、NDVI/NDRE 植被指数、农气数据、历史对比 |
| **免费额度** | 有免费 tier（有限请求） |
| **付费** | $25/月起 |

### 5.3 Google Earth Engine

| 项目 | 内容 |
|------|------|
| **网址** | https://developers.google.com/earth-engine |
| **功能** | Landsat/Sentinel NDVI 年度/月度合成，30m 分辨率 |
| **费用** | 免费（需申请） |

**与本项目的集成方式：**
1. 用于 Dashboard 展示区域植被长势趋势图
2. NDVI 异常可触发 AI 评估中的作物健康分项
3. 配合 ESP32 土壤数据做综合分析

---

## 6. 农产品行情（低优先级）

| API | 免费额度 | 数据范围 | 适用性 |
|-----|----------|----------|--------|
| **Commodities-API** | 100K 次/月 | 咖啡、大米、小麦、糖、玉米等 | 面向决策者，与基层监控关联度低 |
| **CommodityPriceAPI** | 无限试用 | 130+ 商品 | 同上 |
| **APIFarmer 农产品价格** | 无免费 | 食品、畜牧、金属 | 需付费 |

**评估：** 对本项目的温室环境控制核心目标帮助不大，暂不建议接入。

---

## 7. 本地可部署的开源模型（低优先级）

| 模型 | 来源 | 能力 | 部署方式 |
|------|------|------|----------|
| Plant Disease (InceptionResNetV2) | HuggingFace kero2111/Plant_Disease | 38 类作物病害（苹果、玉米、葡萄、土豆等） | `agri-core` 中用 `tract` / `ort` 加载 ONNX |
| Plant-Disease-Detection | HuggingFace Diginsa/Plant-Disease... | CNN 病害检测 | 同上 |

**注意：** 本地 ML 推理需要设备有足够算力（桌面服务端可行，ESP32 不可行）。

---

## 8. 汇总对比表

| # | API | 类别 | 免费额度 | 集成难度 | 对核心目标的贡献 | 推荐 |
|---|-----|------|----------|----------|------------------|------|
| 1 | **crop.health** (Kindwise) | 病害检测 | 100次/月 | 低（一个代理端点） | ⭐⭐⭐⭐⭐ 直接增强 AI 决策 | ✅ |
| 2 | **Flora API** | 植物数据库 | 1,000次/月 | 低 | ⭐⭐⭐⭐ 知识库自动填充 | ✅ |
| 3 | **Weatherbit AgWeather** | 农业气象 | 有免费 tier | 中（需适配响应格式） | ⭐⭐⭐⭐ 外部数据校验 | ✅ |
| 4 | Agromonitoring | 卫星遥感 | 有免费 tier | 中 | ⭐⭐⭐ 植被长势监测 | 可选 |
| 5 | NASA LANCE NDVI | 卫星遥感 | 免费 | 高（处理 GeoTIFF） | ⭐⭐ 长势趋势 | 可选 |
| 6 | Commodities-API | 行情 | 100K/月 | 低 | ⭐ 与环控无关 | ❌ |
| 7 | Plant Disease (HuggingFace) | 本地模型 | 免费 | 高（ML 推理集成） | ⭐⭐⭐ 离线病害识别 | 远期 |

---

### 建议行动顺序

```
第一周:  crop.health → 新增 /api/v1/ai/detect-disease 端点
第二周:  Flora API   → 集成到 Obsidian 知识库种子填充
第三周:  Weatherbit  → 补充 AgWeather 土壤/蒸散数据
远期:    NDVI 遥感  → 区域植被长势 Dashboard 面板
```

编写日期: 2026-05-22
