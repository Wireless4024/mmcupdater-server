# Minecraft MultiMC-pack updater server
this is server side implementation for [mmcupdater](https://github.com/Wireless4024/mmcupdater)

> It may need rust nightly to build (not sure)

## What is can do?
+ Restart / Stop (start via restart) your minecraft forge server
+ Update forge version at server automatically
+ Update mod and restart automatically
+ Serve mods file via http
  > this method it only needs you to upload mod to your server 
  > then it will do everything for you 
  > (your player just need to restart game)
  + Client can see existing mod at server
  + Client can download mods from your server


## How it work?
1. Load config from [default_server.json](default_server.json)
2. Wait for trigger at [/restart](#restart)
3. Scan mods folder in [minecraft.directory](default_server.json#L4)
4. Store them (global for now)
5. Attempt to start server (please test your server before use)

### Admin
#### configuration
[default_server.json](default_server.json)
```json5
{
  "minecraft": {
    // start script (this will call to start server)
    "script": "start.sh",
    // minecraft directory (absolute path is supported?)
    "directory": "mc",
    // does nothing for now
    "folders": [
      "mods"
    ],
    // which file to exclude from scanning (if you back-up a mods using .disabled)
    "exclude": "(?m).+[^(jar)]$"
  }
}
```

[mc/config.json](mc/config.json) (will send to [client](https://github.com/Wireless4024/mmcupdater#at-server-side))
```json5
{
  // config to send to client side
  "config": {
    // minecraft version
    "mc_version": "1.18.1",
    // forge version
    "forge_version": "39.0.79"
  },
  "mods": [
    // server will send url to client instead of download from server directly
    // you can leave it empty
    {
      // mod name (can be anything that always identical to mod after updated)
      "name": "Just Enough Items",
      // version can be version or hash or anything that different between version
      "version": "9.2.1.99",
      // if file name doesn't prefix with http it will download mod via "http://${server}:${port}/mods/${filename}"
      "file_name": "https://media.forgecdn.net/files/3650/556/jei-1.18.1-9.4.1.99.jar"
    }
  ]
}
```

#### API
for web example see [App.svelte](ui/src/App.svelte)
```shell
# $host is host to server (default is localhost:8888) config are coming soon
# $auth will automatic generate at .env

# restart
curl 'http://$HOST/restart' -H 'Authorization: $auth'

# stop
curl 'http://$HOST/stop' -H 'Authorization: $auth'

# update
# this endpoint will automatic restart server and rescan mod automatically
curl 'http://$HOST/update' -v -F 'file=@path/to/jar'
```

> Note: if request to stop server while it running it will delay for 15 seconds

## Build
Requirement:
+ cargo and rustup
+ node and npm (if you need ui)
```shell
./build please
```
or if you only want to run and test it
```shell
cargo run --package mmcupdater-server --bin mmcupdater-server
```