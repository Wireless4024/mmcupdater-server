# Minecraft MultiMC-pack updater server
this is server side implementation for [mmcupdater](https://github.com/Wireless4024/mmcupdater)

## How it work?
1. Load config from [default_server.json](default_server.json)
2. Wait for trigger at [/restart](#restart)
3. Scan mods folder in [minecraft.directory](default_server.json#L4)
4. Store them (global for now)
5. Attempt to start server (please test your server before use)

### Admin
#### configuration
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
```shell
# $host is host to server (default is localhost:4776) config are coming soon
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
```shell
cargo build --release
```
or if you only want to run and test it
```shell
cargo run --package mmcupdater-server --bin mmcupdater-server
```