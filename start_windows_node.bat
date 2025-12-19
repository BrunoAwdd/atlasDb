@echo off
title AtlasDB Node (Auto-Connect)
color 0A

:: --- CONFIGURACAO DO BOOTSTRAP (SEU PC) ---
set BOOT_IP=192.168.15.133
set BOOT_ID=12D3KooWBJ9CPECt4hcJzuMrZJeFXjNGRzaeJ5XfArYRA3hqC8tz
:: ------------------------------------------

echo ==========================================
echo      AtlasDB Auto-Launcher (Windows)
echo ==========================================
echo.
echo Tentando liberar Firewall...
netsh advfirewall firewall add rule name="AtlasDB Node In" dir=in action=allow program="%~dp0atlas-node.exe" enable=yes profile=any >nul 2>&1
netsh advfirewall firewall add rule name="AtlasDB Node Out" dir=out action=allow program="%~dp0atlas-node.exe" enable=yes profile=any >nul 2>&1
if %errorlevel% neq 0 (
    echo [AVISO] Nao foi possivel adicionar regra no Firewall automaticamente.
    echo         Se o no nao conectar, execute este script como ADMINISTRADOR.
    timeout /t 3 >nul
)

echo Detectando IP local...

:: Pega o primeiro IP IPv4 que aparecer no ipconfig (Geralmente eh o da rede local wlan/eth)
for /f "tokens=2 delims=:" %%a in ('ipconfig ^| findstr "IPv4"') do (
    set IP=%%a
    goto :found_ip
)

:found_ip
:: Remove espacos em branco do inicio
set IP=%IP: =%
echo Seu IP detectado: %IP%
echo Bootstrap Alvo:   %BOOT_IP%

if "%IP%"=="%BOOT_IP%" (
    echo.
    echo [MODO MASTER] Voce eh o Bootstrap Node!
    echo.
    "%~dp0atlas-node.exe" --listen /ip4/%IP%/tcp/4001 --grpc-port 50051 --config config.json --keypair node-key
) else (
    echo [MODO PEER] Conectando automaticamente ao Mestre...
    echo.
    "%~dp0atlas-node.exe" --listen /ip4/%IP%/tcp/4001 --grpc-port 50051 --dial /ip4/%BOOT_IP%/tcp/4001/p2p/%BOOT_ID% --config config.json --keypair node-key
)

echo.
echo O no parou de rodar.
pause
goto :eof

:error
echo.
echo ERRO: Informacoes invalidas. Tente novamente.
pause
exit /b

:end
echo.
echo O no parou de rodar.
pause
