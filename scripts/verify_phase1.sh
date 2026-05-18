#!/bin/bash
# 验证第一阶段 API

BASE="http://localhost:3000/api/v1"

echo "=== 第一阶段 API 验证 ==="

# 1. 创建区域
echo -e "\n1. 创建区域"
curl -s -X POST "$BASE/areas" \
  -H "Content-Type: application/json" \
  -d '{"name": "番茄大棚A区", "description": "种植番茄的区域"}' | python3 -m json.tool

# 2. 创建作物（带舒适区间）
echo -e "\n2. 创建作物"
curl -s -X POST "$BASE/crops" \
  -H "Content-Type: application/json" \
  -d '{"name": "番茄", "comfort_config": {"temperature": {"min": 15, "max": 30}, "humidity": {"min": 40, "max": 80}}}' | python3 -m json.tool

# 3. 列出区域
echo -e "\n3. 列出区域"
curl -s "$BASE/areas" | python3 -m json.tool

# 4. 列出作物
echo -e "\n4. 列出作物"
curl -s "$BASE/crops" | python3 -m json.tool

# 5. 创建茬口（需要先获取area_id和crop_id）
echo -e "\n5. 创建茬口"
AREA_ID=$(curl -s "$BASE/areas" | python3 -c "import sys,json; data=json.load(sys.stdin); print(data[0]['id'] if data else '')")
CROP_ID=$(curl -s "$BASE/crops" | python3 -c "import sys,json; data=json.load(sys.stdin); print(data[0]['id'] if data else '')")
if [ -n "$AREA_ID" ] && [ -n "$CROP_ID" ]; then
  curl -s -X POST "$BASE/crop-batches" \
    -H "Content-Type: application/json" \
    -d "{\"area_id\": \"$AREA_ID\", \"crop_id\": \"$CROP_ID\", \"plant_date\": $(date +%s)}" | python3 -m json.tool
fi

# 6. 列出茬口
echo -e "\n6. 列出茬口"
curl -s "$BASE/crop-batches" | python3 -m json.tool

echo -e "\n=== 验证完成 ==="
