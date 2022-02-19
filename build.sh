#!/bin/bash

if [[ "$1" == "please" ]]; then
  rustup --help
  if [[ "$?" != "0" ]]; then
    echo rustup not found installing
    curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly  --profile minimal -y
    source "$HOME/.cargo/env"
  fi
fi

rustup toolchain install nightly
cargo +nightly build --release

mkdir -p dist/mc
chmod +x target/release/mmcupdater-server
cp target/release/mmcupdater-server dist/
cp .env.default dist/.env
cp default_server.json dist/

cp mc/{config.json,start.sh,user_jvm_args.txt} dist/mc

echo npm version
npm -version

if [[ "$?" != "0" ]]; then
  echo you need npm to build ui but it\'s optional
  exit 1
fi

cd ui
npm install
npm run build
rm public/build/*.map

cd ..
mkdir dist/web
cp -r ui/public/* dist/web