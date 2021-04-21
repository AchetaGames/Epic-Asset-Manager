#!/bin/bash

current=$(grep -Po "version: '\K([0-9]*\.[0-9]*.[0-9]+)(?=')" meson.build)
id=$(grep -Po "base_id\s+=\s+'\K(.*)(?=')" meson.build)
major=$(cut -d '.' -f1 <<< "$current")
minor=$(cut -d '.' -f2 <<< "$current")
patch=$(cut -d '.' -f3 <<< "$current")

case $1 in
    major)
        next=$((major + 1)).0.0
        ;;
    minor)
        next="$major".$((minor + 1)).0
        ;;
    patch)
        next="$major"."$minor".$((patch + 1))
        ;;
    *)
        echo "Don't know what to do, exiting!"
        exit 1
    ;;
esac

sed -i "s/version: '$current'/version: '$next'/" meson.build
sed -i "s/version = \"$current\"/version = \"$next\"/" Cargo.toml
sed -i "/<releases>/a\ \ \ \ <release version=\"$next\" date=\"$(date +%F)\">\n\ \ \ \ \ \ \ <description>\n\ \ \ \ \ \ \ \ \ \ \ \ \ \ <p><\!\-\- release:$next --></p>\n\ \ \ \ \ \ \ </description>\n\ \ \ \ </release>" data/"$id".metainfo.xml.in.in
line=$(grep -n "<p><\!\-\- release:$next --></p>" data/"$id".metainfo.xml.in.in | cut -d : -f 1)
sed -i "s|<p><\!\-\- release:$next --></p>|<p></p>|" data/"$id".metainfo.xml.in.in

${EDITOR:=nano} +"$line""$( [ "$EDITOR" == "nano" ] && echo ",18" )" data/"$id".metainfo.xml.in.in

ninja -C _build test

git commit -av
git tag v"$next"

ninja -C _build release
