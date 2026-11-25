# Mixed favorites for the combined Favorites shelf preview: real labels where we have
# them (Genesis, Intellivision, arcade, Game & Watch), named stubs for GBA and SNES
# so each still wears its own shell.
set -eu
fitted="$HOME/Projects/slot-cart-renders/fitted"
gw="$HOME/sbp-pr25/brick/labels-local/GW"

root="$1"

put() {
    platform="$1"; ext="$2"; stem="$3"; label="$4"
    mkdir -p "$root/Games/$platform" "$root/Labels/$platform"
    printf 'stand-in\n' > "$root/Games/$platform/$stem.$ext"
    if [ -f "$label" ]; then
        cp "$label" "$root/Labels/$platform/$stem.png"
    fi
}

put GBA gba "Metroid Fusion" ""
put SNES sfc "Super Metroid" ""
put MD md "Gunstar Heroes (USA)" "$fitted/MD/Gunstar Heroes (USA).png"
put INTV int "Astrosmash (1981) (Mattel)" "$fitted/INTV/Astrosmash (1981) (Mattel).png"
put ARCADE zip "mslug" "$fitted/ARCADE/mslug.png"
stem="Donkey Kong II (Nintendo, Multi Screen)"
put GW mgw "$stem" "$gw/$stem.png"
if [ -f "$gw/open/$stem.png" ]; then
    mkdir -p "$root/Labels/GW/open"
    cp "$gw/open/$stem.png" "$root/Labels/GW/open/$stem.png"
fi

mkdir -p "$root/Config"
# Newest star first on ADDED: Intellivision, then Genesis, SNES, arcade, GBA, G&W.
cat > "$root/Config/library.txt" << 'LIB'
GBA	Metroid Fusion	0	0	1
SNES	Super Metroid	0	80	1
MD	Gunstar Heroes (USA)	0	50	1
INTV	Astrosmash (1981) (Mattel)	0	0	1
ARCADE	mslug	0	10	1
GW	Donkey Kong II (Nintendo, Multi Screen)	0	0	1
FAVADD	INTV	Astrosmash (1981) (Mattel)	40
FAVADD	MD	Gunstar Heroes (USA)	30
FAVADD	SNES	Super Metroid	20
FAVADD	ARCADE	mslug	15
FAVADD	GBA	Metroid Fusion	10
FAVADD	GW	Donkey Kong II (Nintendo, Multi Screen)	5
LIB
