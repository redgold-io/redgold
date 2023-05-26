
# This script mostly keeps track of debug info for installing perf

https://medium.com/lumen-engineering-blog/tutorial-profiling-cpu-and-ram-usage-of-rust-micro-services-running-on-kubernetes-fbc32714da93

docker exec -it redgold-dev /bin/bash

apt-get update && apt-get install -y linux-tools-generic
#ln -s /usr/lib/linux-tools/5.4.0-149-generic/perf /usr/bin/perf
# already exists

WARNING: perf not found for kernel 5.4.0-110

  You may need to install the following packages for this specific kernel:
    linux-tools-5.4.0-110-generic
    linux-cloud-tools-5.4.0-110-generic

apt-get update && apt-get install -y linux-tools-5.4.0-110-generic

