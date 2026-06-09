#!/bin/bash
# SSH 配置恢复脚本（宿主机重启后执行）
set -euo pipefail

SSH_DIR="$HOME/.ssh"
mkdir -p "$SSH_DIR"

# 恢复私钥
if [ -f "/root/agri-iot/config/ssh/id_ed25519" ]; then
    cp /root/agri-iot/config/ssh/id_ed25519 "$SSH_DIR/id_ed25519"
    chmod 600 "$SSH_DIR/id_ed25519"
    echo "✓ 私钥已恢复"
else
    echo "! 私钥文件缺失，需重新生成"
fi

# 恢复公钥
if [ -f "/root/agri-iot/config/ssh/id_ed25519.pub" ]; then
    cp /root/agri-iot/config/ssh/id_ed25519.pub "$SSH_DIR/id_ed25519.pub"
    echo "✓ 公钥已恢复"
fi

# 恢复 config
if [ -f "/root/agri-iot/config/ssh/config" ]; then
    cp /root/agri-iot/config/ssh/config "$SSH_DIR/config"
    chmod 600 "$SSH_DIR/config"
    echo "✓ SSH config 已恢复"
fi

# 恢复 known_hosts
if [ -f "/root/agri-iot/config/ssh/known_hosts" ]; then
    cp /root/agri-iot/config/ssh/known_hosts "$SSH_DIR/known_hosts"
    echo "✓ known_hosts 已恢复"
fi

echo "=== SSH 配置恢复完成 ==="
