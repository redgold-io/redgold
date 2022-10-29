/*
# For x64 CPUs:
sudo docker run -it \
--name nextcloud-aio-mastercontainer \
--restart always \
-p 81:80 \
-p 8081:8080 \
-p 8444:8443 \
--volume nextcloud_aio_mastercontainer:/mnt/docker-aio-config \
--volume /var/run/docker.sock:/var/run/docker.sock:ro \
nextcloud/all-in-one:latest



Please do not forget to open port 3478/TCP and 3478/UDP in your firewall/router for the Talk container!

sudo ufw allow proto tcp from any to any port 81
sudo ufw allow proto tcp from any to any port 8081
sudo ufw allow proto tcp from any to any port 8444

sudo ufw allow proto udp from any to any port 3478
sudo ufw allow proto tcp from any to any port 3478


sudo ufw allow proto tcp from any to any port 16181

*/