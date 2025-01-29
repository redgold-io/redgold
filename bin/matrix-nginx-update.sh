# scp root@n1.redgold.io:/matrix/nginx-proxy/conf.d/matrix-base-domain.conf src/resources/infra/matrix-base-domain.conf
 scp src/resources/infra/matrix-base-domain.conf root@n1.redgold.io:/matrix/nginx-proxy/conf.d/matrix-base-domain.conf
 scp src/resources/infra/traefik-provider.yml root@n1.redgold.io:/matrix/traefik/config/provider.yml
 ssh root@n1.redgold.io "bash -c 'docker restart matrix-nginx-proxy'"