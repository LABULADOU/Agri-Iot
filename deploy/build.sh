#!/bin/bash
set -e

echo "Building Agri-IoT System..."

# 构建后端
cargo build --release -p agri-server

# 构建前端 (React + Vite)
if command -v npm &> /dev/null; then
    cd agri-ui
    npm install
    npm run build
    cd ..
    echo "Frontend built successfully"
else
    echo "npm not found, skipping frontend build"
fi

echo "Build complete!"
echo "Binary: target/release/agri-server"
