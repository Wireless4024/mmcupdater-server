# Minecraft MultiMC-pack updater server

this is server side implementation for [mmcupdater](https://github.com/Wireless4024/mmcupdater)

# WIP - Re-implement its internal design

> Currently, it doesn't work

## Plan (unordered)
+ [ ] (ui) implement nav bar & mobile nav
+ [ ] (ui) allow custom theme
+ [ ] implement api & ui for instance
+ [ ] implement api & ui for java
+ [ ] implement api & ui for mod
+ [ ] integrate curseforge api
+ [ ] improve cli 
+ [ ] support socket.io protocol 
+ [ ] rework db cache
+ [ ] rework db usage
+ [ ] rework mod scanning code
+ [ ] rework eval & process handling
+ [ ] resource check before launch (e.g. available memory)
+ [ ] one-click proxy [backend, api, ui] to do automatic configuration & routing
+ [ ] improve configuration in file
+ [ ] if all ui stuff size <5MiB pack it into binary
+ [ ] hot-swap instance (add & remove from file system on the fly)
+ [ ] move proxy impl to pedestal
+ [ ] websocket support (maybe grpc instead) / event sourcing
+ [ ] remove unused code (fix all warning)
+ [ ] remove duplicate code
+ [ ] implement docs for ui
+ [ ] multiple-node support (executable itself can run as client or server mode)
+ [ ] fast file transfer between nodes
+ [ ] (ui) node manager