# Optional fitted labels for a nicer local preview. CI does not run this.
# Missing files are skipped, so the preview still works without them.
set -eu
root="$1"
fitted="${HOME}/Projects/slot-cart-renders/fitted"
gw="${HOME}/sbp-pr25/brick/labels-local/GW"

copy_label() {
    platform="$1"; stem="$2"; src="$3"
    if [ -f "$src" ]; then
        mkdir -p "$root/Labels/$platform"
        cp "$src" "$root/Labels/$platform/$stem.png"
    fi
}

copy_label MD "Gunstar Heroes (USA)" "$fitted/MD/Gunstar Heroes (USA).png"
copy_label INTV "Astrosmash (1981) (Mattel)" "$fitted/INTV/Astrosmash (1981) (Mattel).png"
copy_label ARCADE mslug "$fitted/ARCADE/mslug.png"
stem="Donkey Kong II (Nintendo, Multi Screen)"
copy_label GW "$stem" "$gw/$stem.png"
if [ -f "$gw/open/$stem.png" ]; then
    mkdir -p "$root/Labels/GW/open"
    cp "$gw/open/$stem.png" "$root/Labels/GW/open/$stem.png"
fi
