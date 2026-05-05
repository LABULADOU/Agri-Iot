#!/bin/bash
set -e

# 设置环境变量
export RUST_LOG=${RUST_LOG:-agri_server=info,agri_mqtt=info}

# 启动服务
echo "Starting Agri-IoT Server..."
exec ./target/release/agri-server
