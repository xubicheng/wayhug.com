@echo off
REM Wayhug AI Engine 开机自启脚本（由 Trae 配置）
cd /d D:\jshnp\wayhug.com
set "PATH=C:\Program Files\nodejs;%PATH%"
node src\server.js >> logs\server.log 2>&1
