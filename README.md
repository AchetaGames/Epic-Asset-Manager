<p align="center">
<a href="https://discord.gg/dumxVnYe6n">
    <img alt="Discord" src="https://img.shields.io/discord/332629362094374913"></a>
    <img alt="Build Status" src="https://github.com/AchetaGames/Epic-Asset-Manager/actions/workflows/rust.yml/badge.svg">
</p>

**This version is obsolete, kept only for those not able to use gtk4, please use the flatpak if at all posible**

# Epic-Asset-Manager
A frontend to Assets purchased on Epic Games Store

## Current Screenshot
![Screenshot from 2021-04-05 03-15-03](https://user-images.githubusercontent.com/252905/113527240-2bbb8800-95bd-11eb-8580-60711816fd21.png)

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
 - Install dependencies: **gtk3 libsoup**
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

## Action video 
[![Youtube Video](https://img.youtube.com/vi/mF0RGK5LglE/maxresdefault.jpg)](https://youtu.be/mF0RGK5LglE)
