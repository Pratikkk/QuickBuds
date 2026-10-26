package com.spizganed.quickbuds.ui

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.PorterDuff
import android.graphics.PorterDuffColorFilter
import android.graphics.RectF
import android.graphics.drawable.Drawable
import android.view.MotionEvent
import android.view.View
import android.view.animation.DecelerateInterpolator
import com.spizganed.quickbuds.widget.QuickBudsWidget

/**
 * The pill position at [fraction] of a move from [from] to [to], clamped to the track. [last] is
 * the last segment index. The spring overshoots, so a fraction above 1 is valid: the pill then
 * stays on the edge and never leaves the track.
 */
internal fun segmentPosition(from: Float, to: Float, fraction: Float, last: Int): Float =
    (from + (to - from) * fraction).coerceIn(0f, last.toFloat())

/**
 * True when the view has a selection. [selected] is the segment index; -1 means no selection.
 * The animated position does not decide this: a move to segment 0 passes below 0.
 */
internal fun hasSelection(selected: Int): Boolean = selected >= 0

/**
 * Noise-control switcher: a card-style container (22dp radius, 4dp padding)
 * holding equal-width 62dp segments, each a 22dp icon over an 11.5sp label. The accent fill SLIDES
 * to the active segment. A tap reports the segment index through [onSegmentTapped]; the fill only
 * moves when [selected] is set — i.e. when the buds' state says so. [selected] = -1 is the neutral
 * (disconnected) state: no fill. With no icons (Dev Tools' Simple / Detailed tabs) it is a 48dp text-only switch.
 */
class AncSegmentedView(
    context: Context,
    labels: List<String>,
    iconRes: List<Int> = emptyList()
) : View(context) {

    var onSegmentTapped: ((Int) -> Unit)? = null

    var selected: Int = -1
        set(v) {
            if (v == field) return
            val from = field
            field = v
            // A replaced animator must not move pos: the new one starts from the current frame.
            anim?.apply {
                removeAllUpdateListeners()
                removeAllListeners()
                cancel()
            }
            appearAnim?.cancel()
            if (p.glass && !nothing && width > 0) {
                // Glass: the pill appears and leaves with a pop instead of snapping, also from and to "nothing chosen".
                if (v < 0) { animateAppear(0f); return }
                if (from < 0) { pos = v.toFloat(); animateAppear(1f); return }
                appear = 1f
            } else appear = if (v < 0) 0f else 1f
            if (width == 0 || from < 0 || v < 0) { pos = v.toFloat(); invalidate(); return }
            val start = pos
            val last = labels.size - 1
            val target = v.toFloat().coerceIn(0f, last.toFloat())
            anim = ValueAnimator.ofFloat(0f, 1f).apply {
                duration = 380
                // The pill overshoots its target a little and settles: quick, with a spring.
                interpolator = android.view.animation.OvershootInterpolator(1.1f)
                addUpdateListener {
                    pos = segmentPosition(start, target, it.animatedValue as Float, last)
                    postInvalidateOnAnimation()
                }
                addListener(object : AnimatorListenerAdapter() {
                    override fun onAnimationEnd(animation: Animator) {
                        // End on the target, also after a cancel with no replacement.
                        pos = target
                        postInvalidateOnAnimation()
                    }
                })
                start()
            }
        }

    private var pos = -1f
    private var anim: ValueAnimator? = null

    /** Glass: how much of the pill shows, 0 (nothing chosen) to 1. Overshoots a little on its way in. */
    private var appear = 0f
    private var appearAnim: ValueAnimator? = null

    private fun animateAppear(to: Float) {
        appearAnim = ValueAnimator.ofFloat(appear, to).apply {
            duration = if (to > 0f) 320 else 180
            interpolator = if (to > 0f) android.view.animation.OvershootInterpolator(1.6f) else android.view.animation.DecelerateInterpolator()
            addUpdateListener { appear = it.animatedValue as Float; postInvalidateOnAnimation() }
            start()
        }
    }

    private fun dp(v: Float) = ThemeRes.dp(context, v).toFloat()

    private val p = ThemeRes.palette(context)
    private val labels = labels.toMutableList()
    private val icons: MutableList<Drawable> = iconRes.map { ThemeRes.depth(context, context.getDrawable(it)!!.mutate()) }.toMutableList()

    /**
     * Nothing style: the widget's dot-matrix mode icons ([QuickBudsWidget.modeIcon]), white, tinted when drawn.
     * Rendered at a whole-pixel pitch (about 1.15dp) and drawn unscaled: shrunk from the widget's 8 px pitch to
     * 28dp, each dot fell on fractional pixels and smeared.
     */
    private val dotPitch = Math.round(dp(1.15f)).coerceAtLeast(2).toFloat()
    private val dots: MutableList<Bitmap?> =
        if (ThemeRes.nothing(context)) iconRes.map { QuickBudsWidget.modeIcon(WIDGET_ICON[it] ?: it, dotPitch) }.toMutableList() else mutableListOf()

    /** Changes segment [i]'s label and icon (the home ANC segment shows the current level, as the widget's button). */
    fun setSegment(i: Int, label: String, iconRes: Int) {
        if (i !in labels.indices) return
        labels[i] = label
        if (i in icons.indices) icons[i] = ThemeRes.depth(context, context.getDrawable(iconRes)!!.mutate())
        if (i in dots.indices) dots[i] = QuickBudsWidget.modeIcon(WIDGET_ICON[iconRes] ?: iconRes, dotPitch)
        invalidate()
    }
    private val dotPaint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG)

    private val trackPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = if (ThemeRes.material(context)) p.container else p.card }
    private val strokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE; strokeWidth = dp(1f); color = p.outline
    }
    private val pillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = p.accent }
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        textSize = dp(if (iconRes.isEmpty()) 14f else 11.5f); textAlign = Paint.Align.CENTER
        typeface = ThemeRes.medium(context)
    }
    private val box = RectF()
    private val capBounds = android.graphics.Rect()

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        // Nothing: taller, for the bigger unscaled dot icons.
        if (material) { setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), dp(if (icons.isEmpty()) 44f else if (iconOnly) 56f else 72f).toInt()); return }
        setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), dp(if (icons.isEmpty()) 48f else if (dots.isNotEmpty()) 72f else 62f).toInt())
    }

    private val nothing = ThemeRes.nothing(context)

    /** Glass: icons only, no labels (the Left / Right buds). */
    private val iconOnly = icons.isNotEmpty() && labels.all { it.isEmpty() }

    /**
     * Material style. With icons: round 52dp toggle buttons with the label under each (the noise control).
     * Text only: equal 12dp tiles with a 4dp gap (a choice group). The selected one fills with the accent;
     * every option keeps one shape. The fill fades between options instead of sliding.
     */
    private val material = ThemeRes.material(context)

    private val plate = ThemeRes.GlassDrawable(context, 24f, flat = true)
    private val litPaint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val haloPaint = Paint(Paint.ANTI_ALIAS_FLAG)

    /**
     * Glass: one plate with the card's corner radius. A lit glass pill glides under the selected mode (it follows [pos]);
     * every mode is an icon and a label on the plate.
     */
    private fun drawGlassPlate(canvas: Canvas) {
        val n = labels.size
        val inset = dp(4f)
        val segW = (width - inset * 2) / n
        plate.setBounds(0, 0, width, height)
        plate.draw(canvas)
        textPaint.getTextBounds("H", 0, 1, capBounds)
        // The shared clamp in segmentPosition keeps the pill on the plate while it springs.
        val shown = if (appear <= 0.01f && !hasSelection(selected)) -1f else pos
        val a = appear.coerceIn(0f, 1f)
        if (shown >= 0f && a > 0f) {
            val left = inset + segW * shown
            box.set(left, inset, left + segW, height - inset)
            val rad = if (icons.isEmpty()) height / 2f - inset else dp(20f)
            // Flat and bright, the colour of a switch that is on: it reads as "on" like the switches below do.
            val grow = 0.88f + 0.12f * appear
            box.inset(box.width() * (1f - grow) / 2f, box.height() * (1f - grow) / 2f)
            litPaint.shader = null; litPaint.style = Paint.Style.FILL
            litPaint.color = Palette.withAlpha(p.accent, 0.95f * a)
            canvas.drawRoundRect(box, rad, rad, litPaint)
        }
        if (iconOnly) {
            // Icons only: each glyph keeps its own ratio, centred in its segment.
            for (i in 0 until n) {
                val on = if (shown < 0f) 0f else (1f - kotlin.math.abs(shown - i)).coerceIn(0f, 1f) * a
                val cx = inset + segW * i + segW / 2
                icons.getOrNull(i)?.run {
                    setTint(Palette.blend(p.textSecondary, p.onAccent, on))
                    val h = dp(30f)
                    val w = if (intrinsicHeight > 0) h * intrinsicWidth / intrinsicHeight else h
                    setBounds((cx - w / 2).toInt(), ((height - h) / 2).toInt(), (cx + w / 2).toInt(), ((height + h) / 2).toInt())
                    draw(canvas)
                }
            }
            return
        }
        val textOnly = icons.isEmpty()
        val iconS = if (textOnly) 0f else dp(12f)
        val block = if (textOnly) capBounds.height().toFloat() else iconS * 2 + dp(5f) + capBounds.height()
        val iconTop = (height - block) / 2
        for (i in 0 until n) {
            val on = if (shown < 0f) 0f else (1f - kotlin.math.abs(shown - i)).coerceIn(0f, 1f) * a
            val cx = inset + segW * i + segW / 2
            val tint = Palette.blend(p.textSecondary, p.onAccent, on)
            icons.getOrNull(i)?.run {
                setTint(tint)
                setBounds((cx - iconS).toInt(), iconTop.toInt(), (cx + iconS).toInt(), (iconTop + iconS * 2).toInt())
                draw(canvas)
            }
            textPaint.color = tint
            canvas.drawText(labels[i], cx, if (textOnly) iconTop + capBounds.height() else iconTop + iconS * 2 + dp(5f) + capBounds.height(), textPaint)
        }
    }

    private fun drawMaterial(canvas: Canvas) {
        if (p.glass) { drawGlassPlate(canvas); return }
        val n = labels.size
        val gap = dp(4f)
        val segW = (width - gap * (n - 1)) / n
        val idle = Palette.blend(p.container, p.text, 0.08f)
        textPaint.getTextBounds("H", 0, 1, capBounds)
        for (i in 0 until n) {
            val on = if (!hasSelection(selected)) 0f else (1f - kotlin.math.abs(pos - i)).coerceIn(0f, 1f)
            val left = i * (segW + gap)
            val cx = left + segW / 2
            trackPaint.color = Palette.blend(if (icons.isEmpty()) p.container else idle, p.accent, on)
            if (icons.isEmpty()) {
                box.set(left, 0f, left + segW, height.toFloat())
                canvas.drawRoundRect(box, dp(12f), dp(12f), trackPaint)
                textPaint.color = Palette.blend(p.textSecondary, p.onAccent, on)
                canvas.drawText(labels[i], cx, height / 2f + capBounds.height() / 2f, textPaint)
                continue
            }
            val r = dp(26f)
            canvas.drawCircle(cx, r, r, trackPaint)
            icons.getOrNull(i)?.run {
                setTint(Palette.blend(p.textSecondary, p.onAccent, on))
                val s = dp(12f).toInt()
                setBounds((cx - s).toInt(), (r - s).toInt(), (cx + s).toInt(), (r + s).toInt())
                draw(canvas)
            }
            textPaint.color = Palette.blend(p.textSecondary, p.text, on)
            canvas.drawText(labels[i], cx, r * 2 + dp(6f) + capBounds.height(), textPaint)
        }
    }

    /** Track and outline, dots in the dot style (one-cell outline). */
    private fun track(canvas: Canvas) {
        val h = height.toFloat()
        if (nothing) {
            // The outline as the shape in its colour with the track a cell inside, and a radius a little under
            // half the height: a stroke, or a full half-height radius, left a lone dot at each end.
            val pitch = DotArt.pitchPx(context)
            val trackH = (h / pitch).toInt() * pitch
            box.set(0f, 0f, width.toFloat(), trackH)
            val rad = minOf(dp(22f), trackH / 2 * 0.85f)
            canvas.drawRoundRect(box, rad, rad, strokePaint.apply { style = Paint.Style.FILL })
            box.inset(pitch, pitch)
            canvas.drawRoundRect(box, rad - pitch, rad - pitch, trackPaint)
            // The selected segment: an accent outline a cell wide, the track inside (not a fill).
            // Centred in the snapped track on a whole-cell gap, and concentric with its corners.
            if (hasSelection(selected)) {
                val inset = (dp(4f) / pitch).toInt().coerceAtLeast(1) * pitch
                val segW = (width - inset * 2) / labels.size
                box.set(inset + segW * pos, inset, inset + segW * (pos + 1), trackH - inset)
                val pr = minOf(rad - inset, box.height() / 2 * 0.85f)
                canvas.drawRoundRect(box, pr, pr, pillPaint)
                box.inset(pitch, pitch)
                canvas.drawRoundRect(box, pr - pitch, pr - pitch, trackPaint)
            }
            return
        }
        val e = if (nothing) DotArt.pitchPx(context) / 2 else dp(0.5f)
        box.set(e, e, width - e, h - e)
        canvas.drawRoundRect(box, dp(22f), dp(22f), trackPaint)
        strokePaint.strokeWidth = if (nothing) 0f else dp(1f)
        canvas.drawRoundRect(box, dp(22f), dp(22f), strokePaint)
    }

    /** Classic: the sliding accent fill (the dot style draws an outline in [track]). */
    private fun pill(canvas: Canvas) {
        val h = height.toFloat()
        if (hasSelection(selected) && !nothing) {
            val inset = dp(4f)
            val left = inset + (width - inset * 2) / labels.size * pos
            box.set(left, inset, left + (width - inset * 2) / labels.size, h - inset)
            canvas.drawRoundRect(box, dp(18f), dp(18f), pillPaint)
        }
    }

    override fun onDraw(canvas: Canvas) {
        if (material) { drawMaterial(canvas); return }
        val h = height.toFloat()
        if (nothing) DotArt.draw(context, canvas, width, height) { track(it) } else track(canvas)
        pill(canvas)

        val inset = dp(4f)
        val segW = (width - inset * 2) / labels.size

        // The dot icons get more room: 31 dots in 22dp would blur into a grey disc.
        val iconSize = dp(if (dots.isEmpty()) 22f else 28f)
        // Icon, gap and the label's cap height as one block, centred in the height, so the space above the
        // icon and below the label are equal.
        val iconH = dots.firstOrNull { it != null }?.height?.toFloat() ?: iconSize
        textPaint.getTextBounds("H", 0, 1, capBounds)
        val capH = capBounds.height().toFloat()
        val iconTop = Math.round((h - (iconH + dp(5f) + capH)) / 2).toFloat()
        val baseline = if (icons.isEmpty()) h / 2 + capH / 2 else iconTop + iconH + dp(5f) + capH
        labels.forEachIndexed { i, label ->
            // The segment under the moving fill brightens as the fill arrives.
            val closeness = if (!hasSelection(selected)) 0f else (1f - kotlin.math.abs(pos - i)).coerceIn(0f, 1f)
            val c = Palette.blend(p.textSecondary, if (nothing) p.accent else p.onAccent, closeness)
            val cx = inset + segW * i + segW / 2
            val dot = dots.getOrNull(i)
            if (dot != null) {
                dotPaint.colorFilter = PorterDuffColorFilter(c, PorterDuff.Mode.SRC_IN)
                // Unscaled, on whole pixels.
                canvas.drawBitmap(dot, Math.round(cx - dot.width / 2f).toFloat(), iconTop, dotPaint)
            } else icons.getOrNull(i)?.run {
                setTint(c)
                setBounds(
                    (cx - iconSize / 2).toInt(), iconTop.toInt(),
                    (cx + iconSize / 2).toInt(), (iconTop + iconSize).toInt()
                )
                draw(canvas)
            }
            textPaint.color = c
            canvas.drawText(label, cx, baseline, textPaint)
        }
    }

    override fun onTouchEvent(e: MotionEvent): Boolean {
        if (!isEnabled) return false
        if (e.actionMasked == MotionEvent.ACTION_UP) {
            val i = ((e.x / width) * labels.size).toInt().coerceIn(0, labels.size - 1)
            performClick()
            Haptics.commit(this)
            onSegmentTapped?.invoke(i)
        }
        return true
    }

    override fun performClick(): Boolean = super.performClick()

    companion object {
        /** The home screen's noise icons as the widget's mode icons, the ones drawn as dots. */
        private val WIDGET_ICON = mapOf(
            com.spizganed.quickbuds.R.drawable.ic_noise_off to com.spizganed.quickbuds.R.drawable.ic_mode_off,
            com.spizganed.quickbuds.R.drawable.ic_adaptive to com.spizganed.quickbuds.R.drawable.ic_mode_adaptive,
            com.spizganed.quickbuds.R.drawable.ic_transparency to com.spizganed.quickbuds.R.drawable.ic_mode_transparency
        )
    }
}
