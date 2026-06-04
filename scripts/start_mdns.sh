#!/bin/sh
# Start/restart the mDNS advertiser (agri-server.local)
pkill -f "mdns_advertise.py" 2>/dev/null
sleep 1
exec nohup python3 -u /root/agri-iot/scripts/mdns_advertise.py > /var/log/agri-mdns.log 2>&1
