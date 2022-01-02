<p align="center">
    <a href="https://discord.gg/dumxVnYe6n">
        <img alt="Discord" src="https://img.shields.io/discord/332629362094374913">
    </a>
    <a href="https://github.com/AchetaGames/Epic-Asset-Manager/actions/workflows/release.yml">    
        <img alt="Build Status" src="https://github.com/AchetaGames/Epic-Asset-Manager/actions/workflows/release.yml/badge.svg">
    </a>
    <a href="https://flathub.org/apps/details/io.github.achetagames.epic_asset_manager">
        <img alt="Flathub" src="https://img.shields.io/flathub/v/io.github.achetagames.epic_asset_manager">
    </a>
</p>

# Epic-Asset-Manager
A frontend to Assets purchased on Epic Games Store

## Current Screenshot
![Screenshot from 2021-08-19 17-00-44](https://user-images.githubusercontent.com/252905/130092378-c6eee39f-024b-4f06-a6ab-5e73e39c23f9.png)

## Install
### Arch Linux
Use the [AUR package](https://aur.archlinux.org/packages/eam-git)

### Build flatpak
```bash
meson _build --prefix=/usr --reconfigure;

```
### Build from source
 - Install rust using [rustup](https://rustup.rs/)
 - Install the stable toolchain
```bash
rustup install stable
rustup default stable
```
 - Install dependencies: **gtk4 libadwaita**
 - Clone the repository
```bash
git clone git@github.com:AchetaGames/Epic-Asset-Manager.git
```
 - Move into the repository
```bash
cd Epic-Asset-Manager
```
 - Configure the project
```bash
meson _build
```
 - Build the project (the resulting binary is in target/release/epic_asset_manager)
```bash
meson compile -C _build
```
 - Or install the project
```bash
meson install -C _build
```

## Run
Open from your favorite application tray or run the following in a terminal
```
epic_asset_manager
```

## Action video 
[![Youtube Video](https://img.youtube.com/vi/vgy3j03sZns/maxresdefault.jpg)](https://www.youtube.com/watch?v=vgy3j03sZns)
