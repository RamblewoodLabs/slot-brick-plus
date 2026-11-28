# Slot's Python runtime: with SLOT_FB set (run-app.sh), pygame's screen goes to the framebuffer.
import os

if os.environ.get("SLOT_FB"):
    os.environ.setdefault("PYGAME_HIDE_SUPPORT_PROMPT", "1")
    try:
        import slot_fb

        slot_fb.install(os.environ["SLOT_FB"])
    except Exception as e:
        print("slot_fb: not installed: %s" % e, flush=True)
