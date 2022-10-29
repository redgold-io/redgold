// https://netmaker.readthedocs.io/en/master/quick-start.html

/*
// This part cannot be automated easily since it involves a domain registrar? Maybe add an
// API for godaddy?

Create a wildcard A record pointing to the public IP of your VM. As an example, *.netmaker.example.com.
 Alternatively, create records for these specific subdomains:

    dashboard.domain

    api.domain

    broker.domain

 */

/*
ssh root@your-host
sudo apt-get update
sudo apt-get install -y docker.io docker-compose wireguard
sudo ufw allow proto tcp from any to any port 443 && sudo ufw allow 51821:51830/udp
sudo ufw allow proto tcp from any to any port 53
sudo ufw allow proto tcp from any to any port 8883
iptables --policy FORWARD ACCEPT

ip route get 1 | sed -n 's/^.*src \([0-9.]*\) .*$/\1/p'
# 154.27.85.183
# Note this has to be run in a data directory that makes sense
wget -O docker-compose.yml https://raw.githubusercontent.com/gravitl/netmaker/master/compose/docker-compose.traefik.yml
sed -i 's/NETMAKER_BASE_DOMAIN/<your base domain>/g' docker-compose.yml
sed -i 's/SERVER_PUBLIC_IP/<your server ip>/g' docker-compose.yml
sed -i 's/YOUR_EMAIL/<your email>/g' docker-compose.yml

tr -dc A-Za-z0-9 </dev/urandom | head -c 30 ; echo ''
sed -i 's/REPLACE_MASTER_KEY/<your generated key>/g' docker-compose.yml

wget -O /root/mosquitto.conf https://raw.githubusercontent.com/gravitl/netmaker/master/docker/mosquitto.conf



sed -i 's/redgold.app/netmaker.redgold.app/g' docker-compose.yml
sed -i 's/SERVER_PUBLIC_IP/154.27.85.183/g' docker-compose.yml
sed -i 's/YOUR_EMAIL/email@email.com/g' docker-compose.yml


https://github.com/gravitl/netmaker#get-started-in-5-minutes


 */
