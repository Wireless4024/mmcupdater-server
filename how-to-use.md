# How to use
This guide will help you to use `mmcupdater` to manage your server remotely.

Note:
1. I don't have any guild for Windows user at this moment.
2. Release binary (pre-compiled zip) compiled using dynamic 
   link to glibc, if it may core dump or refused to run on your server 
   please clone this repo and follow [guide](readme.md#build)

## Easy method
> may only work for 1.18+
0. assume you are at `dist` folder that contains `mmcupdater-server` executable
1. Open and edit `default_server.json` and and change `java` to correct java executable location
2. [run server](#to-run)
3. edit minecraft version and forge version in `Instance Manager` tab (older version may use different launch method if crashed see ADVANCED setup)
4. click `File Manager` tab and edit `eula.txt` if needed

## Advanced method
0. assume you are at `dist` folder that contains `mmcupdater-server` executable
1. set up minecraft server in `mc` folder (or copy existing minecraft server to this folder)
2. edit minecraft/forge version in `mc/config.json` (required to send to client)
3. edit `script` in `default_server.json` to your launch script like `run.sh`
2. [run server](#to-run)

## To run
1. run `mmcupdater-server` (via `./mmcupdater-server`)
2. visit `http://$server_ip/web/index.html` [(default: http://localhost:8888/web/index.html)](http://localhost:8888/web/index.html)
3. open `.env` file (maybe hidden) and copy text after `auth_code=`
4. at web interface click `turn on admin mode` and use `auth_code` to enable it
5. click restart! (server will start, and you will need to set up client side)