/*

https://github.com/spantaleev/matrix-docker-ansible-deploy

sudo ufw allow proto tcp from any to any port 80
sudo ufw allow proto tcp from any to any port 443
sudo ufw allow proto tcp from any to any port 3478
sudo ufw allow proto udp from any to any port 3478
sudo ufw allow proto tcp from any to any port 5349
sudo ufw allow proto udp from any to any port 5349
sudo ufw allow proto tcp from any to any port 8448
sudo ufw allow 49152:49172/udp

80/tcp: HTTP webserver
443/tcp: HTTPS webserver
3478/tcp: TURN over TCP (used by Coturn)
3478/udp: TURN over UDP (used by Coturn)
5349/tcp: TURN over TCP (used by Coturn)
5349/udp: TURN over UDP (used by Coturn)
8448/tcp: Matrix Federation API HTTPS webserver. In some cases, this may necessary even with federation disabled. Integration Servers (like Dimension) and Identity Servers (like ma1sd) may need to access openid APIs on the federation port.
the range 49152-49172/udp: TURN over UDP

https://github.com/spantaleev/matrix-docker-ansible-deploy/blob/master/docs/configuring-dns.md
SRV 	_matrix-identity._tcp 	10 	0 	443 	matrix.<your-domain>
CNAME 	dimension 	- 	- 	- 	matrix.<your-domain>
CNAME 	jitsi 	- 	- 	- 	matrix.<your-domain>
CNAME 	stats 	- 	- 	- 	matrix.<your-domain>
CNAME 	goneb 	- 	- 	- 	matrix.<your-domain>
CNAME 	sygnal 	- 	- 	- 	matrix.<your-domain>
CNAME 	hydrogen 	- 	- 	- 	matrix.<your-domain>
CNAME 	cinny 	- 	- 	- 	matrix.<your-domain>
CNAME 	buscarron 	- 	- 	- 	matrix.<your-domain>

godaddy automation around this ^ ?


https://github.com/spantaleev/matrix-docker-ansible-deploy/blob/master/docs/configuring-playbook.md

git clone https://github.com/spantaleev/matrix-docker-ansible-deploy.git

cd matrix-docker-ansible-deploy
mkdir inventory/host_vars/matrix.redgold.app
cp examples/vars.yml inventory/host_vars/matrix.redgold.app/vars.yml
# manual edit after

cp examples/hosts inventory/hosts
# manual edit after

apt install ansible
ansible --version
# 2.9.6 doesn't
apt update
apt install -y software-properties-common
add-apt-repository --yes --update ppa:ansible/ansible
apt install -y ansible

ansible-playbook -i inventory/hosts setup.yml --tags=setup-all

apt install pwgen
pwgen -s 64 1
# for secret key


ansible-playbook -i inventory/hosts setup.yml --tags=start

matrix_nginx_proxy_base_domain_serving_enabled: true

matrix_nginx_proxy_base_domain_homepage_template: some path here? i guess

ansible-playbook -i inventory/hosts setup.yml --tags=setup-all

ansible-playbook -i inventory/hosts setup.yml --tags=self-check

ansible-playbook -i inventory/hosts setup.yml --extra-vars='username=<your-username> password=<your-password> admin=<yes|no>' --tags=register-user
ansible-playbook -i inventory/hosts setup.yml --extra-vars='username=debug password=test admin=yes' --tags=register-user
 */
