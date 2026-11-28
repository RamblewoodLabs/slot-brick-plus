# Sourced by the docker builds: the ScreenScraper developer id that Cart Art is built with.
# Taken from SLOT_SS_DEVID / SLOT_SS_DEVPASSWORD when set (the release workflow's secrets),
# else from devid= / devpassword= in a gitignored screenscraper.properties at the repo root or
# in android/ (or the file named by SLOT_SS_PROPERTIES). The values are exported for `docker run -e NAME` and never printed. Without
# them the card still builds, and HOME -> Cart Art says it was built without a developer id.
for ss_file in ${SLOT_SS_PROPERTIES:-} screenscraper.properties android/screenscraper.properties; do
    [ -z "${SLOT_SS_DEVID:-}" ] || break
    [ -f "$ss_file" ] || continue
    SLOT_SS_DEVID=$(sed -n 's/^devid=//p' "$ss_file" | tr -d '\r' | head -n 1)
    SLOT_SS_DEVPASSWORD=$(sed -n 's/^devpassword=//p' "$ss_file" | tr -d '\r' | head -n 1)
done
unset ss_file
if [ -n "${SLOT_SS_DEVID:-}" ] && [ -n "${SLOT_SS_DEVPASSWORD:-}" ]; then
    export SLOT_SS_DEVID SLOT_SS_DEVPASSWORD
    echo "Cart Art: building with a ScreenScraper developer id"
else
    unset SLOT_SS_DEVID SLOT_SS_DEVPASSWORD
    echo "Cart Art: no ScreenScraper developer id (see docs/cart-art.md); building without one"
fi
