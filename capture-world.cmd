@echo off
rem Runs the title so every 60th presented frame is saved, with every render
rem target that frame used (VICTORY_DUMP_TARGETS).
rem
rem Play to the black 3D part, let it sit a few seconds, then close the game
rem window. Files land in D:\vicfork-work\dump.
setlocal
set VICTORY_LOG=info
set VICTORY_CAPTURE=D:\vicfork-work\dump
set VICTORY_CAPTURE_EVERY=60
set VICTORY_DUMP_TARGETS=1
if not exist D:\vicfork-work\dump mkdir D:\vicfork-work\dump
del /q "D:\vicfork-work\dump\*" >nul 2>&1
echo.
echo   capture folder : %VICTORY_CAPTURE%   (every 60th frame + targets)
echo   binary         : D:\vicfork\target\diag\victory.exe
echo.
echo   Play until the 3D intro is black, hold a few seconds, close the window.
echo.
"D:\vicfork\target\diag\victory.exe" run "D:\GTA6\game_scarlett_master.exe" --execute > "D:\vicfork-work\dump-run.log" 2>&1
echo.
echo   done:
dir /b "D:\vicfork-work\dump"
pause
