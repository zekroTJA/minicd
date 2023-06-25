#!/bin/bash

function error {
    echo "error: $1"
    exit 1
}

function check_installed {
    which "$1" > /dev/null 2>&1 \
        || error "'$1' needs to be installed to run this script."
}

function download {
    target=$1
    url=$2

    status_code=$(curl -sLo "$target" "$url" -w "%{http_code}")
    [[ $status_code -lt 200 || $status_code -gt 299 ]] \
        && error "request failed with code $status_code" 
}

# ----------------------------------------------------------------------------------

[ "$(id -u)" == "0" ] \
    || error "this script must be used as root user."

check_installed "curl"

version=$1

case "$(uname -m)" in
    "x86_64")
        arch="x86_64" ;;
    "aarch64")
        arch="aarch64" ;;
    *)
        error "unsupported system architecture."
esac

[ -z "$version" ] && {
    check_installed "jq"
    version=$(curl -sL "https://api.github.com/repos/zekrotja/minicd/releases?per_page=1" \
        | jq -r '.[0].tag_name')
}

set -x

download /usr/local/bin/minicd \
    "https://github.com/zekroTJA/minicd/releases/download/$version/minicd-$version-$arch-unknown-linux-musl"

download /etc/minicd/config.toml \
    "https://raw.githubusercontent.com/zekroTJA/minicd/main/minicd.example.toml"

download /etc/systemd/system/minicd.service \
    "https://raw.githubusercontent.com/zekroTJA/minicd/main/contrib/systemd/minicd.service"

set -e

chmod +x /usr/local/bin/minicd

mkdir -p /etc/minicd
mkdir -p /var/minicd

systemctl daemon-reload
systemctl enable minicd.service
systemctl start minicd.service
systemctl status minicd.service

set +ex

cat <<EOF
----------------------------------------------------------------

MiniCD service has been successfully installed and started.

A default config has been created. Feel free to open it and
configure it to your needs.
$ sudo vim /etc/minicd/config.toml

After that, restart the service.
$ sudo systemctl restart minicd.service
EOF