package plus.slotbrick.slot

import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent

/**
 * Button and axis map for an AYN Odin / Thor-class Android handheld.
 *
 * These devices show up as a standard Android gamepad (Xbox layout: A east,
 * B south, X north, Y west). The Thor's second screen is ignored for now;
 * this mapper only cares about the built-in pad.
 *
 * Slot actions:
 *  LEFT / RIGHT  — move the game highlight
 *  UP / DOWN or L1 / R1 — change system
 *  A / SOUTH — launch (or open the install prompt)
 *  B / BACK — go to the system row / folder picker is long-press BACK
 *  START — ROM folder picker
 *  Y / Select — Settings
 */
enum class SlotAction { LEFT, RIGHT, UP, DOWN, LAUNCH, BACK, PICK_FOLDER, SETTINGS, NONE }

object GamepadMap {
    fun isOdinFamily(device: InputDevice?): Boolean {
        val name = device?.name?.lowercase() ?: return false
        return name.contains("odin") || name.contains("ayn") || name.contains("thor") ||
            name.contains("retroid")
    }

    fun actionFromKey(event: KeyEvent): SlotAction {
        if (event.action != KeyEvent.ACTION_DOWN) return SlotAction.NONE
        if (event.repeatCount > 0 && event.keyCode != KeyEvent.KEYCODE_DPAD_LEFT &&
            event.keyCode != KeyEvent.KEYCODE_DPAD_RIGHT &&
            event.keyCode != KeyEvent.KEYCODE_DPAD_UP &&
            event.keyCode != KeyEvent.KEYCODE_DPAD_DOWN
        ) {
            return SlotAction.NONE
        }
        return when (event.keyCode) {
            KeyEvent.KEYCODE_DPAD_LEFT, KeyEvent.KEYCODE_BUTTON_L2 -> SlotAction.LEFT
            KeyEvent.KEYCODE_DPAD_RIGHT, KeyEvent.KEYCODE_BUTTON_R2 -> SlotAction.RIGHT
            KeyEvent.KEYCODE_DPAD_UP, KeyEvent.KEYCODE_BUTTON_L1, KeyEvent.KEYCODE_BUTTON_THUMBL -> SlotAction.UP
            KeyEvent.KEYCODE_DPAD_DOWN, KeyEvent.KEYCODE_BUTTON_R1, KeyEvent.KEYCODE_BUTTON_THUMBR -> SlotAction.DOWN
            KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_BUTTON_1, KeyEvent.KEYCODE_ENTER,
            KeyEvent.KEYCODE_DPAD_CENTER -> SlotAction.LAUNCH
            KeyEvent.KEYCODE_BUTTON_B, KeyEvent.KEYCODE_BACK, KeyEvent.KEYCODE_BUTTON_2 -> SlotAction.BACK
            KeyEvent.KEYCODE_BUTTON_START, KeyEvent.KEYCODE_MENU -> SlotAction.PICK_FOLDER
            KeyEvent.KEYCODE_BUTTON_Y, KeyEvent.KEYCODE_BUTTON_SELECT, KeyEvent.KEYCODE_BUTTON_4 -> SlotAction.SETTINGS
            else -> SlotAction.NONE
        }
    }

    /** Hat-switch D-pad on many Odin / Xbox-style pads. Returns a one-shot action. */
    fun actionFromHat(event: MotionEvent, lastX: Float, lastY: Float): Pair<SlotAction, Pair<Float, Float>> {
        val x = axis(event, MotionEvent.AXIS_HAT_X)
        val y = axis(event, MotionEvent.AXIS_HAT_Y)
        val action = when {
            lastX > -0.5f && x <= -0.5f -> SlotAction.LEFT
            lastX < 0.5f && x >= 0.5f -> SlotAction.RIGHT
            lastY > -0.5f && y <= -0.5f -> SlotAction.UP
            lastY < 0.5f && y >= 0.5f -> SlotAction.DOWN
            else -> SlotAction.NONE
        }
        return action to (x to y)
    }

    fun leftStick(event: MotionEvent): Pair<Float, Float> {
        return axis(event, MotionEvent.AXIS_X) to axis(event, MotionEvent.AXIS_Y)
    }

    fun rightStick(event: MotionEvent): Pair<Float, Float> {
        val z = axis(event, MotionEvent.AXIS_Z)
        val rz = axis(event, MotionEvent.AXIS_RZ)
        if (kotlin.math.abs(z) > 0.01f || kotlin.math.abs(rz) > 0.01f) return z to rz
        return axis(event, MotionEvent.AXIS_RX) to axis(event, MotionEvent.AXIS_RY)
    }

    private fun axis(event: MotionEvent, axis: Int): Float {
        val device = event.device ?: return 0f
        val range = device.getMotionRange(axis, event.source) ?: return event.getAxisValue(axis)
        val v = event.getAxisValue(axis)
        val flat = range.flat
        return if (kotlin.math.abs(v) < flat) 0f else v
    }
}
