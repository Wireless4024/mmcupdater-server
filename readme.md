# Minecraft MultiMC-pack updater server
this is server side implementation for [mmcupdater](https://github.com/Wireless4024/mmcupdater)

# WIP - Re-implement its internal design
> Currently, it doesn't work

## Build
Requirement:
+ cargo and rustup
+ node and npm (if you need ui, download from latest release should be ok)
> if your machine is weak please edit last section of [Cargo.toml](Cargo.toml) to this
> (it will make build process faster)
> ```toml
> [profile.release]
> opt-level = 1
> lto = "off"
>  ```
```shell
./build please
# after build finish, you only need `dist` folder to use
```
or if you only want to run and test it
```shell
cargo run --package mmcupdater-server --bin mmcupdater-server
```