# Shelf filters: GBA and arcade games the offline rankings know (and one they don't), as
# stand-ins, so SELECT's TOP 100 / POPULAR / BEST SELLING / FIRST PARTY can be screenshotted.
# Real labels are optional, added by brick/sim/filters-preview-labels.sh on a machine that has them.
set -eu
root="$1"

put() {
    platform="$1"; ext="$2"; stem="$3"
    mkdir -p "$root/Games/$platform"
    printf 'stand-in\n' > "$root/Games/$platform/$stem.$ext"
}

for stem in \
    "Advance Wars (USA) (Rev 1)" \
    "Castlevania - Aria of Sorrow (USA)" \
    "Doom (USA, Europe)" \
    "Golden Sun (USA, Europe)" \
    "Legend of Zelda, The - The Minish Cap (USA)" \
    "Mario Kart - Super Circuit (USA)" \
    "Max Payne (USA)" \
    "Metroid - Zero Mission (USA)" \
    "Metroid Fusion (USA)" \
    "Pokemon - Emerald Version (USA, Europe)" \
    "Pokemon - FireRed Version (USA, Europe) (Rev 1)" \
    "Sonic Advance (USA) (En,Ja)" \
    "Super Mario Advance (USA, Europe)" \
    "Tony Hawk's Pro Skater 2 (USA, Europe)" \
    "WarioWare, Inc. - Mega Microgame\$! (USA)" \
    "Yoshi Topsy-Turvy (USA)"
do
    put GBA gba "$stem"
done
for name in 1942 dkong galaga kof98 mslug outrun pacman sf2 simpsons tmnt; do
    put ARCADE zip "$name"
done
