#!/bin/bash
set -euo pipefail

cd ../web
pnpm run build

cd ..
cargo build --release

cd docker

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' ../Cargo.toml)
echo "Extracted version: $VERSION"

cp ../target/release/mailboxd-server ./amd64/mailboxd-server 2>/dev/null \
    || { mkdir -p ./amd64 && cp ../target/release/mailboxd-server ./amd64/mailboxd-server; }
cp ../target/release/mailboxd-cli ./amd64/mailboxd-cli
cp ../target/release/mailboxd-admin ./amd64/mailboxd-admin
cp ../LICENSE ./amd64/LICENSE
cp ../NOTICE ./amd64/NOTICE 2>/dev/null || true

docker build --build-arg CRATE_VERSION="$VERSION" --build-arg TARGETARCH=amd64 -t "mailboxd:$VERSION" .
echo "Built mailboxd:$VERSION"
