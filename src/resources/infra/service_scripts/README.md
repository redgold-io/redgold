cp testnet-canary /etc/systemd/system/redgold-testnet-canary.service
systemctl daemon-reload
journalctl -u redgold-testnet-canary.service  -b | tail -n 100

systemctl restart redgold-testnet-canary.service
systemctl status redgold-testnet-canary.service
systemctl status redgold-testnet.service

systemctl restart redgold-testnet.service
