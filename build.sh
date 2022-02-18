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

mkdir dist
mv target/release/mmcupdater-server dist

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