//! Fluent durations and cubic-bezier curves.

use egui::{Context, Id};
use fluent2_tokens::generated::global::{curve, duration};

const REDUCED_MOTION_KEY: &str = "fluent2_reduced_motion";
const NEWTON_STEPS: u32 = 8;

/// Fluent `duration*` tokens, as seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Duration {
    /// `durationUltraFast` (50 ms).
    UltraFast,
    /// `durationFaster` (100 ms).
    Faster,
    /// `durationFast` (150 ms).
    Fast,
    /// `durationNormal` (200 ms).
    Normal,
    /// `durationGentle` (250 ms).
    Gentle,
    /// `durationSlow` (300 ms).
    Slow,
    /// `durationSlower` (400 ms).
    Slower,
    /// `durationUltraSlow` (500 ms).
    UltraSlow,
}

impl Duration {
    /// Token length in seconds.
    #[must_use]
    pub fn seconds(self) -> f32 {
        let millis = match self {
            Self::UltraFast => duration::ULTRA_FAST,
            Self::Faster => duration::FASTER,
            Self::Fast => duration::FAST,
            Self::Normal => duration::NORMAL,
            Self::Gentle => duration::GENTLE,
            Self::Slow => duration::SLOW,
            Self::Slower => duration::SLOWER,
            Self::UltraSlow => duration::ULTRA_SLOW,
        };
        millis as f32 / 1000.0
    }
}

/// Fluent `curve*` tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Curve {
    /// `curveAccelerateMax`.
    AccelerateMax,
    /// `curveAccelerateMid`.
    AccelerateMid,
    /// `curveAccelerateMin`.
    AccelerateMin,
    /// `curveDecelerateMax`.
    DecelerateMax,
    /// `curveDecelerateMid`.
    DecelerateMid,
    /// `curveDecelerateMin`.
    DecelerateMin,
    /// `curveEasyEase`.
    EasyEase,
    /// `curveEasyEaseMax`.
    EasyEaseMax,
    /// `curveLinear`.
    Linear,
}

impl Curve {
    fn bezier(self) -> [f32; 4] {
        match self {
            Self::AccelerateMax => curve::ACCELERATE_MAX.0,
            Self::AccelerateMid => curve::ACCELERATE_MID.0,
            Self::AccelerateMin => curve::ACCELERATE_MIN.0,
            Self::DecelerateMax => curve::DECELERATE_MAX.0,
            Self::DecelerateMid => curve::DECELERATE_MID.0,
            Self::DecelerateMin => curve::DECELERATE_MIN.0,
            Self::EasyEase => curve::EASY_EASE.0,
            Self::EasyEaseMax => curve::EASY_EASE_MAX.0,
            Self::Linear => curve::LINEAR.0,
        }
    }

    /// Evaluate the cubic-bezier easing at `t`.
    ///
    /// `t` is clamped to `0..=1`. Non-finite input returns `0.0`. The X root uses at most eight
    /// Newton iterations.
    #[must_use]
    pub fn ease(self, t: f32) -> f32 {
        if !t.is_finite() {
            return 0.0;
        }
        let t = t.clamp(0.0, 1.0);
        if t == 0.0 || t == 1.0 {
            return t;
        }
        let [x1, y1, x2, y2] = self.bezier();
        let x = solve_bezier_x(t, x1, x2);
        sample_bezier(x, y1, y2)
    }
}

/// Store whether motion should snap to its target.
pub fn set_reduced_motion(ctx: &Context, enabled: bool) {
    ctx.data_mut(|data| data.insert_temp(reduced_motion_id(), enabled));
}

/// Whether [`set_reduced_motion`] asked motion to snap. Callers that drive their own clock
/// (not [`animate`]) check this.
#[must_use]
pub fn reduced_motion(ctx: &Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(reduced_motion_id()).unwrap_or(false))
}

/// Animate `target` over `duration` with `curve`.
///
/// [`Context::animate_value_with_time`] moves linearly. This samples `curve` on that progress.
/// When `target` changes before the run settles, the clock is reseeded at the value returned
/// last frame (the eased value, not egui's linear one) and the new run starts there. Reduced
/// motion returns `target` immediately.
pub fn animate(ctx: &Context, id: Id, target: f32, duration: Duration, curve: Curve) -> f32 {
    if !target.is_finite() {
        return 0.0;
    }
    if reduced_motion(ctx) {
        return target;
    }
    let from_id = id.with("fluent2_motion_from");
    let to_id = id.with("fluent2_motion_to");
    let eased_id = id.with("fluent2_motion_eased");
    let previous_to = ctx.data(|data| data.get_temp::<f32>(to_id));
    let previous_eased = ctx.data(|data| data.get_temp::<f32>(eased_id));
    let stored_from = ctx.data(|data| data.get_temp::<f32>(from_id));
    let retargeted = previous_to.is_some_and(|previous| (previous - target).abs() > f32::EPSILON);
    if retargeted {
        let current = previous_eased.unwrap_or(target);
        // egui would otherwise restart from its last *linear* return. Seed that stored value
        // with the eased sample, then aim the same clock at the new target.
        ctx.animate_value_with_time(id, current, 0.0);
        ctx.data_mut(|data| data.insert_temp(from_id, current));
    }
    let origin = ctx.data(|data| data.get_temp::<f32>(from_id).unwrap_or(target));
    let linear = ctx.animate_value_with_time(id, target, duration.seconds());
    let span = target - origin;
    let progress = if span.abs() <= f32::EPSILON {
        1.0
    } else {
        ((linear - origin) / span).clamp(0.0, 1.0)
    };
    let eased = if stored_from.is_none() && !retargeted {
        // First observation: egui reports `target` immediately, so there is no span yet.
        // Remember it as the origin; the next frame eases away from it.
        ctx.data_mut(|data| {
            data.insert_temp(from_id, linear);
            data.insert_temp(to_id, target);
            data.insert_temp(eased_id, linear);
        });
        return linear;
    } else {
        origin + span * curve.ease(progress)
    };
    ctx.data_mut(|data| {
        data.insert_temp(from_id, origin);
        data.insert_temp(to_id, target);
        data.insert_temp(eased_id, eased);
    });
    eased
}

fn reduced_motion_id() -> Id {
    Id::NULL.with(REDUCED_MOTION_KEY)
}

/// Solve `bezier_x(u) = t` for `u` in `0..=1`.
fn solve_bezier_x(t: f32, x1: f32, x2: f32) -> f32 {
    let mut u = t;
    for _ in 0..NEWTON_STEPS {
        let x = sample_bezier(u, x1, x2) - t;
        let dx = bezier_slope(u, x1, x2);
        if dx.abs() < 1e-6 {
            break;
        }
        u = (u - x / dx).clamp(0.0, 1.0);
    }
    if !u.is_finite() { t } else { u }
}

fn sample_bezier(u: f32, c1: f32, c2: f32) -> f32 {
    let u2 = u * u;
    let u3 = u2 * u;
    let one = 1.0 - u;
    let one2 = one * one;
    3.0 * one2 * u * c1 + 3.0 * one * u2 * c2 + u3
}

fn bezier_slope(u: f32, c1: f32, c2: f32) -> f32 {
    let one = 1.0 - u;
    3.0 * one * one * c1 + 6.0 * one * u * (c2 - c1) + 3.0 * u * u * (1.0 - c2)
}

#[cfg(test)]
mod tests {
    use super::Curve;

    #[test]
    fn ease_endpoints_are_identity() {
        for curve in [
            Curve::AccelerateMax,
            Curve::AccelerateMid,
            Curve::AccelerateMin,
            Curve::DecelerateMax,
            Curve::DecelerateMid,
            Curve::DecelerateMin,
            Curve::EasyEase,
            Curve::EasyEaseMax,
            Curve::Linear,
        ] {
            assert_eq!(curve.ease(0.0), 0.0);
            assert_eq!(curve.ease(1.0), 1.0);
        }
    }

    #[test]
    fn easy_ease_is_monotonic() {
        let mut previous = 0.0;
        let mut t = 0.0;
        while t <= 1.0 {
            let value = Curve::EasyEase.ease(t);
            assert!(value + 1e-4 >= previous, "{value} < {previous} at {t}");
            previous = value;
            t += 0.05;
        }
    }

    #[test]
    fn non_finite_and_out_of_range_are_finite() {
        for t in [-1.0, f32::NAN, 2.0] {
            let value = Curve::EasyEase.ease(t);
            assert!(value.is_finite(), "{t} -> {value}");
        }
        assert_eq!(Curve::EasyEase.ease(-1.0), 0.0);
        assert_eq!(Curve::EasyEase.ease(2.0), 1.0);
    }
}
