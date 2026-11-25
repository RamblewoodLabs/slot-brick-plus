# Mixed favorites for the combined Favorites shelf: a stub game on each of
# several systems, already starred, so the shelf can be screenshotted in CI.
# Real labels are optional and added by brick/sim/run-favorites-preview.sh
# when those files are on the machine; they are not required here.
set -eu
root="$1"

put() {
    platform="$1"; ext="$2"; stem="$3"
    mkdir -p "$root/Games/$platform"
    printf 'stand-in\n' > "$root/Games/$platform/$stem.$ext"
}

put GBA gba "Metroid Fusion"
put SNES sfc "Super Metroid"
put MD md "Gunstar Heroes (USA)"
put INTV int "Astrosmash (1981) (Mattel)"
put ARCADE zip "mslug"
put GW mgw "Donkey Kong II (Nintendo, Multi Screen)"

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
