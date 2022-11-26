#!/bin/sh

server_file=''
java=java

if [ "$1" ]; then
    server_file="$1"
fi

if [ "$2" ]; then
    java="$2"
fi

if [[ "$current_version" != "$server_file" ]]; then
    $java -jar $server_file --installServer
fi

script_file="libraries/net/minecraftforge/forge/$ver/unix_args.txt"
script=`echo $(< $script_file)`

$java @user_jvm_args.txt $script nogui