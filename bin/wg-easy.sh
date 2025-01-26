docker stop wg-easy; docker rm wg-easy; docker run --detach \
--name wg-easy \
--env LANG=en \
--env WG_HOST=5.161.103.120 \
--env PASSWORD_HASH=$PASSWORD_HASH \
--env PORT=51821 \
--env WG_PORT=51820 \
--volume ~/.wg-easy:/etc/wireguard \
--publish 51820:51820/udp \
--publish 51821:51821/tcp \
--publish 8080:8080/tcp \
--publish 16479:16479/tcp \
--publish 16480:16480/tcp \
--publish 16481:16481/tcp \
--publish 16482:16482/tcp \
--publish 16483:16483/tcp \
--publish 16484:16484/tcp \
--publish 16485:16485/tcp \
--publish 16486:16486/tcp \
--publish 16487:16487/tcp \
--publish 16179:16179/tcp \
--publish 16180:16180/tcp \
--publish 16181:16181/tcp \
--publish 16182:16182/tcp \
--publish 16183:16183/tcp \
--publish 16184:16184/tcp \
--publish 16185:16185/tcp \
--publish 16186:16186/tcp \
--publish 16187:16187/tcp \
--publish 16279:16279/tcp \
--publish 16280:16280/tcp \
--publish 16281:16281/tcp \
--publish 16282:16282/tcp \
--publish 16283:16283/tcp \
--publish 16284:16284/tcp \
--publish 16285:16285/tcp \
--publish 16286:16286/tcp \
--publish 16287:16287/tcp \
--publish 16379:16379/tcp \
--publish 16380:16380/tcp \
--publish 16381:16381/tcp \
--publish 16382:16382/tcp \
--publish 16383:16383/tcp \
--publish 16384:16384/tcp \
--publish 16385:16385/tcp \
--publish 16386:16386/tcp \
--publish 16387:16387/tcp \
--cap-add NET_ADMIN \
--cap-add SYS_MODULE \
--sysctl 'net.ipv4.conf.all.src_valid_mark=1' \
--sysctl 'net.ipv4.ip_forward=1' \
--restart unless-stopped \
ghcr.io/wg-easy/wg-easy

#!/bin/bash
ports=(8080 16479 16480 16481 16482 16483 16484 16485 16486 16487 16179 16180 16181 16182 16183 16184 16185 16186 16187 16279 16280 16281 16282 16283 16284 16285 16286 16287 16379 16380 16381 16382 16383 16384 16385 16386 16387)

for port in "${ports[@]}"; do
    docker exec wg-easy iptables -t nat -A PREROUTING -p tcp --dport $port -j DNAT --to-destination 10.8.0.2:$port
    docker exec wg-easy iptables -A FORWARD -p tcp -d 10.8.0.2 --dport $port -j ACCEPT
done

docker exec wg-easy iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE


# client
wg-quick down wg0 && wg-quick up wg0 && curl ifconfig.me