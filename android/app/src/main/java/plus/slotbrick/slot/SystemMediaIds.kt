package plus.slotbrick.slot

/**
 * Maps a Slot system id to a ScreenScraper `systemeid` and a libretro-thumbnails
 * playlist folder. IDs follow the public ScreenScraper system list / ES-DE map.
 */
object SystemMediaIds {
    data class MediaIds(
        val screenscraperId: Int?,
        val libretroPlaylist: String?,
    )

    private val MAP = mapOf(
        "gb" to MediaIds(9, "Nintendo - Game Boy"),
        "gbc" to MediaIds(10, "Nintendo - Game Boy Color"),
        "gba" to MediaIds(12, "Nintendo - Game Boy Advance"),
        "nes" to MediaIds(3, "Nintendo - Nintendo Entertainment System"),
        "snes" to MediaIds(4, "Nintendo - Super Nintendo Entertainment System"),
        "n64" to MediaIds(14, "Nintendo - Nintendo 64"),
        "nds" to MediaIds(15, "Nintendo - Nintendo DS"),
        "n3ds" to MediaIds(17, "Nintendo - Nintendo 3DS"),
        "gc" to MediaIds(13, "Nintendo - GameCube"),
        "gw" to MediaIds(52, "Nintendo - Game & Watch"),
        "switch" to MediaIds(225, "Nintendo - Nintendo Switch"),
        "md" to MediaIds(1, "Sega - Mega Drive - Genesis"),
        "genesis" to MediaIds(1, "Sega - Mega Drive - Genesis"),
        "sms" to MediaIds(2, "Sega - Master System - Mark III"),
        "gg" to MediaIds(21, "Sega - Game Gear"),
        "32x" to MediaIds(19, "Sega - 32X"),
        "s32x" to MediaIds(19, "Sega - 32X"),
        "saturn" to MediaIds(22, "Sega - Saturn"),
        "dreamcast" to MediaIds(23, "Sega - Dreamcast"),
        "psx" to MediaIds(57, "Sony - PlayStation"),
        "ps1" to MediaIds(57, "Sony - PlayStation"),
        "ps2" to MediaIds(58, "Sony - PlayStation 2"),
        "psp" to MediaIds(61, "Sony - PlayStation Portable"),
        "psvita" to MediaIds(62, "Sony - PlayStation Vita"),
        "vita" to MediaIds(62, "Sony - PlayStation Vita"),
        "pce" to MediaIds(31, "NEC - PC Engine - TurboGrafx 16"),
        "tg16" to MediaIds(31, "NEC - PC Engine - TurboGrafx 16"),
        "a2600" to MediaIds(26, "Atari - 2600"),
        "atari2600" to MediaIds(26, "Atari - 2600"),
        "a5200" to MediaIds(40, "Atari - 5200"),
        "atari5200" to MediaIds(40, "Atari - 5200"),
        "a7800" to MediaIds(41, "Atari - 7800"),
        "atari7800" to MediaIds(41, "Atari - 7800"),
        "atari800" to MediaIds(43, "Atari - 8-bit"),
        "a800" to MediaIds(43, "Atari - 8-bit"),
        "lynx" to MediaIds(28, "Atari - Lynx"),
        "coleco" to MediaIds(48, "Coleco - ColecoVision"),
        "intv" to MediaIds(115, "Mattel - Intellivision"),
        "intellivision" to MediaIds(115, "Mattel - Intellivision"),
        "vectrex" to MediaIds(102, "GCE - Vectrex"),
        "arcade" to MediaIds(75, "FBNeo - Arcade Games"),
        "fbneo" to MediaIds(75, "FBNeo - Arcade Games"),
        "neogeo" to MediaIds(142, "FBNeo - Arcade Games"),
        "xbox" to MediaIds(32, "Microsoft - Xbox"),
    )

    fun of(systemId: String): MediaIds {
        MAP[systemId]?.let { return it }
        MAP[systemId.lowercase()]?.let { return it }
        return MediaIds(null, null)
    }
}
