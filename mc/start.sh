#!/bin/bash

forge_installer_file='forge-1.18.1-39.0.79-installer.jar'
java=java

if [ "$1" ]; then
    forge_installer_file="forge-$1-installer.jar"

fi

if [ "$2" ]; then
    java="$2"
fi

if [ ! -f "version" ]; then
    echo '--' > version
fi

current_version=`echo $( < version)`

ver="${forge_installer_file::-14}"
ver="${ver#forge-}"
if [ ! -f "forge_installer_file" ]; then
    curl -O -L "https://maven.minecraftforge.net/net/minecraftforge/forge/$ver/$forge_installer_file"
fi

if [[ "$current_version" != "$forge_installer_file" ]]; then
    $java -jar $forge_installer_file --installServer
fi

script_file="libraries/net/minecraftforge/forge/$ver/unix_args.txt"
script=`echo $(< $script_file)`

$java @user_jvm_args.txt $script nogui