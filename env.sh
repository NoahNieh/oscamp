#!/usr/bin/bash

sudo apt update && sudo apt install -y make cmake build-essential dosfstools
if [ $? -ne 0 ]; then
    exit 1
fi


# rustup
type cargo > /dev/null 2>&1
if [ $? -ne 0 ]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    . "$HOME/.cargo/env"
fi

# rust utils
cargo install cargo-binutils
if [ $? -ne 0 ]; then
    exit 1
fi

# qemu
sudo apt install -y qemu-system

# sanity check

cat > env_info << EOF
-----info-----------
qemu: $(qemu-system-riscv64 --version | head -n1)
cargo: $(cargo --version)
mkfs.fat: $(mkfs.fat --help | head -n1)
EOF

cat env_info 




