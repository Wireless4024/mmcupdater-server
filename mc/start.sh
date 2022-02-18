#!/bin/bash

# start script

java --add-exports java.base/sun.security.util=ALL-UNNAMED --add-opens java.base/java.util=ALL-UNNAMED --add-opens java.base/java.util.jar=ALL-UNNAMED \
-XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=40 -XX:G1ReservePercent=1 \
-XX:G1HeapRegionSize=2M -XX:+TieredCompilation -XX:MaxTenuringThreshold=2 -XX:+OptimizeStringConcat \
-XX:+ParallelRefProcEnabled -XX:+AlwaysPreTouch -XX:+UseNUMA -XX:-UseStringDeduplication -XX:-G1UseAdaptiveIHOP \
-XX:G1HeapWastePercent=10 -XX:G1MixedGCCountTarget=4 -XX:G1MixedGCLiveThresholdPercent=70 -XX:InitiatingHeapOccupancyPercent=70 \
-XX:G1RSetUpdatingPauseTimePercent=5 -XX:SurvivorRatio=32 -XX:+PerfDisableSharedMem -server \
-XX:FreqInlineSize=63 -XX:InlineSmallCode=11174 -XX:MaxInlineLevel=2239 -XX:MaxInlineSize=24 -XX:MaxRecursiveInlineLevel=186 -XX:MinInliningThreshold=40 \
-XX:+UseJVMCINativeLibrary -XX:+UseJVMCICompiler -XX:MaxGCPauseMillis=200 -Xms512M -Xmx4G \
-jar minecraft_server.jar nogui