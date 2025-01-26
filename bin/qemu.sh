
mkdir -p ~/qemu-vms/ubuntu
cd ~/qemu-vms/ubuntu

sudo apt update
sudo apt install qemu-system-x86 qemu-utils  # On Debian/Ubuntu-based systems

qemu-img create -f qcow2 ubuntu.qcow2 20G
wget https://releases.ubuntu.com/jammy/ubuntu-22.04.5-live-server-amd64.iso

qemu-system-x86_64 \
-enable-kvm \
-m 16G \
-boot d \
-drive file=ubuntu.qcow2,format=qcow2 \
-cdrom ubuntu-22.04.5-live-server-amd64.iso \
-nographic \
-serial mon:stdio

qemu-system-x86_64 \
  -enable-kvm \
  -m 2G \
  -drive file=ubuntu.qcow2,format=qcow2 \
  -net nic -net user,hostfwd=tcp::2222-:22 \
  -nographic

ssh -p 2222 <your-ubuntu-username>@localhost

mkdir -p ~/qemu-share

qemu-system-x86_64 \
  -enable-kvm \
  -m 2G \
  -drive file=ubuntu.qcow2,format=qcow2 \
  -net nic -net user,hostfwd=tcp::2222-:22 \
  -nographic \
  -virtfs local,path=~/qemu-share,mount_tag=shared9p,security_model=passthrough,id=hostshare

sudo mkdir -p /mnt/hostshare
sudo mount -t 9p -o trans=virtio,version=9p2000.L shared9p /mnt/hostshare
echo "shared9p  /mnt/hostshare  9p  trans=virtio,version=9p2000.L  0 0" | sudo tee -a /etc/fstab
ssh -p 2222 <username>@localhost
