# Optional labels for a nicer local preview of the shelf filters. CI does not run this.
# Missing files are skipped, so the preview still works without them.
set -eu
root="$1"
gba="${HOME}/Projects/slot-rankings/labels/GBA"
fitted="${HOME}/Projects/slot-cart-renders/fitted"
for dir in Games/GBA Games/ARCADE; do
    [ -d "$root/$dir" ] || continue
    platform=$(basename "$dir")
    for game in "$root/$dir"/*; do
        stem=$(basename "$game"); stem="${stem%.*}"
        for src in "$gba/$stem.png" "$fitted/$platform/$stem.png"; do
            if [ -f "$src" ]; then
                mkdir -p "$root/Labels/$platform"
                cp "$src" "$root/Labels/$platform/$stem.png"
                break
            fi
        done
    done
done
