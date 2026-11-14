// Mush8 - motion.rs
// Constantin Clerc (c22dev) - 0.1
// Fusion is a Rust port of MotionInput from the yuzu Emulator Project (src/hid_core/frontend/motion_input.cpp)
// SPDX-FileCopyrightText: Copyright 2020 yuzu Emulator Project
// SPDX-License-Identifier: GPL-2.0-or-later
const KP: f32 = 0.3;
const KI: f32 = 0.005;
const GYRO_THRESHOLD: f32 = 0.007;
const AT_REST: f32 = 0.05;
const GYRO_MAX: f32 = 5.0;
const ACCEL_MAX: f32 = 7.0;

type V3 = [f32; 3];

fn len(v: V3) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

#[derive(Clone, Copy)]
pub struct Fusion {
    quat: [f32; 4],
    accel: V3,
    gyro: V3,
    gyro_bias: V3,
    real_error: V3,
    integral_error: V3,
    rotation: V3,
    only_accel: bool,
}

impl Default for Fusion {
    fn default() -> Self {
        Fusion {
            quat: [0.0, 0.0, 0.0, -1.0],
            accel: [0.0; 3],
            gyro: [0.0; 3],
            gyro_bias: [0.0; 3],
            real_error: [0.0; 3],
            integral_error: [0.0; 3],
            rotation: [0.0; 3],
            only_accel: true,
        }
    }
}

impl Fusion {
    pub fn update(&mut self, accel: V3, gyro: V3, dt: f32) {
        self.accel = accel.map(|a| a.clamp(-ACCEL_MAX, ACCEL_MAX));
        self.gyro = std::array::from_fn(|i| (gyro[i] - self.gyro_bias[i]).clamp(-GYRO_MAX, GYRO_MAX));
        if !self.moving(AT_REST) {
            self.gyro_bias = std::array::from_fn(|i| self.gyro_bias[i] * 0.9999 + gyro[i] * 0.0001);
        }
        if len(self.gyro) < GYRO_THRESHOLD {
            self.gyro = [0.0; 3];
        } else {
            self.only_accel = false;
        }
        self.rotate(dt);
        self.orient(dt);
    }

    fn moving(&self, sensitivity: f32) -> bool {
        let a = len(self.accel);
        len(self.gyro) >= sensitivity || a <= 0.9 || a >= 1.1
    }

    fn rotate(&mut self, dt: f32) {
        if dt <= 0.1 {
            self.rotation = std::array::from_fn(|i| self.rotation[i] + self.gyro[i] * dt);
        }
    }

    fn orient(&mut self, dt: f32) {
        if dt > 0.1 {
            return;
        }
        let [mut q1, q2, q3, q4] = self.quat;
        let tau = std::f32::consts::TAU;
        let mut w = if self.only_accel { [0.0; 3] } else { [self.gyro[1] * tau, -self.gyro[0] * tau, -self.gyro[2] * tau] };
        let a = len(self.accel);
        if (0.75..=1.25).contains(&a) {
            let n = self.accel.map(|v| v / a);
            let (ax, ay, az) = (-n[0], n[1], -n[2]);
            let vx = 2.0 * (q2 * q4 - q1 * q3);
            let vy = 2.0 * (q1 * q2 + q3 * q4);
            let vz = q1 * q1 - q2 * q2 - q3 * q3 + q4 * q4;
            self.real_error = [az * vx - ax * vz, ay * vz - az * vy, ax * vy - ay * vx];
            if len(self.real_error) >= 0.05 {
                self.integral_error = std::array::from_fn(|i| self.integral_error[i] + self.real_error[i]);
            } else {
                self.integral_error = [0.0; 3];
            }
            let (kp, ki) = if self.only_accel { (35.0 * KP, 10.0 * KI) } else { (KP, KI) };
            for i in 0..3 {
                w[i] += kp * self.real_error[i] + ki * self.integral_error[i];
            }
            if self.only_accel {
                self.gyro = [-w[1], w[0], -w[2]];
                self.rotate(dt);
            }
        }
        let (gx, gy, gz) = (w[1], w[0], w[2]);
        let h = 0.5 * dt;
        q1 += (-q2 * gx - q3 * gy - q4 * gz) * h;
        let q = [q1, q2 + (q1 * gx + q3 * gz - q4 * gy) * h, q3 + (q1 * gy - q2 * gz + q4 * gx) * h, q4 + (q1 * gz + q2 * gy - q3 * gx) * h];
        let l = q.iter().map(|v| v * v).sum::<f32>().sqrt();
        self.quat = q.map(|v| v / l);
    }

    pub fn gyro(&self) -> V3 {
        self.gyro
    }

    pub fn rotation(&self) -> V3 {
        self.rotation
    }

    pub fn orientation(&self) -> [V3; 3] {
        let [qw, qx, qy, qz] = self.quat;
        let (x, y, z, w) = (-qy, -qx, -qw, -qz);
        let m = [
            [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y + w * z), 2.0 * (x * z - w * y)],
            [2.0 * (x * y - w * z), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z + w * x)],
            [2.0 * (x * z + w * y), 2.0 * (y * z - w * x), 1.0 - 2.0 * (x * x + y * y)],
        ];
        [[m[0][0], m[0][1], -m[0][2]], [m[1][0], m[1][1], -m[1][2]], [-m[2][0], -m[2][1], m[2][2]]]
    }
}

/// Gravity on the phone's X axis needs to be at least this large to pick the landscape side.
pub const PHONE_SIDE_LOCK: f32 = 0.35;

/// Turns one phone sample into the pad frame and smooths it.
///
/// A phone held upright in landscape puts gravity on ±X of the device. A real controller held like
/// a wheel (face towards the player) reads gravity on its -Y axis, and the game steers from that:
/// rolling the controller clockwise reads as gravity moving towards -X, which is a right turn. With
/// `held` set, the rotation below reproduces the frame a real controller reports in the same pose
/// (gravity on -Y, same X axis), so rolling the phone like a wheel steers like a real pad.
/// `held = false` keeps the earlier screen-oriented mapping, which put gravity on -Z while playing
/// and steered inverted.
///
/// Which of the two landscape sides the phone is in is latched from gravity, so the mapping does
/// not flip while the phone is nearly flat.
#[derive(Clone, Copy)]
pub struct PhoneFilter {
    tilt_tau: f32,
    turn_tau: f32,
    accel: V3,
    gyro: V3,
    stamp: f32,
    side: f32,
    held: bool,
    ready: bool,
}

impl PhoneFilter {
    /// `tilt_tau` and `turn_tau` are low-pass time constants in seconds, `0.0` disables smoothing.
    /// `held` selects the held-controller frame; see the type comment.
    pub fn new(tilt_tau: f32, turn_tau: f32, held: bool) -> Self {
        PhoneFilter { tilt_tau, turn_tau, accel: [0.0; 3], gyro: [0.0; 3], stamp: f32::NAN, side: 1.0, held, ready: false }
    }

    /// `accel` is gravity plus user acceleration in g, `gyro` the rotation rate in rad/s, both in
    /// the phone's frame; `stamp` is when the sample was taken, in seconds.
    pub fn update(&mut self, accel: V3, gyro: V3, stamp: f32) -> (V3, V3) {
        if accel[0].abs() > PHONE_SIDE_LOCK {
            self.side = accel[0].signum();
        }
        let side = self.side;
        let held = self.held;
        let map = |v: V3| match (held, side > 0.0) {
            (true, true) => [v[1], -v[0], v[2]],
            (true, false) => [-v[1], v[0], v[2]],
            (false, true) => [-v[1], v[2], -v[0]],
            (false, false) => [v[1], v[2], v[0]],
        };
        let (accel, gyro) = (map(accel), map(gyro));
        let dt = if self.stamp.is_finite() && stamp.is_finite() { (stamp - self.stamp).clamp(0.0, 0.25) } else { 0.0 };
        self.stamp = stamp;
        if !self.ready {
            self.ready = true;
            self.accel = accel;
            self.gyro = gyro;
            return (accel, gyro);
        }
        let k = |tau: f32| if tau > 0.0 && dt > 0.0 { dt / (tau + dt) } else { 1.0 };
        let (kt, kr) = (k(self.tilt_tau), k(self.turn_tau));
        self.accel = std::array::from_fn(|i| self.accel[i] + (accel[i] - self.accel[i]) * kt);
        self.gyro = std::array::from_fn(|i| self.gyro[i] + (gyro[i] - self.gyro[i]) * kr);
        (self.accel, self.gyro)
    }
}

#[derive(Clone, Copy)]
pub enum Orientation {
    Pad(Fusion),
    Phone(Fusion),
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::Phone(Fusion::default())
    }
}

impl Orientation {
    pub fn update(&mut self, accel: V3, gyro: V3, pad: bool, dt: f32) {
        match (self, pad) {
            (Orientation::Pad(f), true) | (Orientation::Phone(f), false) => f.update(accel, gyro, dt),
            (o, _) => {
                *o = if pad { Orientation::Pad(Fusion::default()) } else { Orientation::Phone(Fusion::default()) };
                o.update(accel, gyro, pad, dt);
            }
        }
    }

    pub fn rotation(&self) -> V3 {
        match self {
            Orientation::Pad(f) | Orientation::Phone(f) => f.rotation(),
        }
    }

    pub fn orientation(&self) -> [V3; 3] {
        match self {
            Orientation::Pad(f) | Orientation::Phone(f) => f.orientation(),
        }
    }

    pub fn gyro(&self) -> V3 {
        match self {
            Orientation::Pad(f) | Orientation::Phone(f) => f.gyro(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(f: &mut Fusion, n: usize, accel: [f32; 3], gyro: [f32; 3]) {
        for _ in 0..n {
            f.update(accel, gyro, 0.008333);
        }
    }

    fn close(a: &[f32], b: &[f32]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
    }

    #[test]
    fn phones_use_the_gravity_fusion() {
        let mut o = Orientation::default();
        let mut f = Fusion::default();
        for _ in 0..120 {
            o.update([0.0, 0.0, -1.0], [0.25, 0.0, 0.0], false, 0.008333);
            f.update([0.0, 0.0, -1.0], [0.25, 0.0, 0.0], 0.008333);
        }
        assert!(close(o.orientation().as_flattened(), f.orientation().as_flattened()));
        assert!(close(&o.gyro(), &f.gyro()));
    }

    #[test]
    fn gamepads_use_fusion() {
        let mut o = Orientation::default();
        let mut f = Fusion::default();
        for _ in 0..120 {
            o.update([0.0, 0.0, -1.0], [0.25, 0.0, 0.0], true, 0.008333);
            f.update([0.0, 0.0, -1.0], [0.25, 0.0, 0.0], 0.008333);
        }
        assert!(close(o.orientation().as_flattened(), f.orientation().as_flattened()));
        assert!(close(&o.gyro(), &f.gyro()));
    }

    #[test]
    fn phone_upright_landscape_reads_like_a_held_pad() {
        for gx in [-0.98f32, 0.98] {
            let mut f = PhoneFilter::new(0.0, 0.0, true);
            let (accel, _) = f.update([gx, 0.02, -0.19], [0.0; 3], 0.0);
            assert!((accel[1] + 0.98).abs() < 0.01, "gravity should be on -Y like a held pad, got {accel:?}");
            assert!(accel[2].abs() < 0.25, "the screen normal should be near horizontal, got {accel:?}");
            assert!(accel[0].abs() < 0.05, "level pad should read no roll, got {accel:?}");
        }
    }

    #[test]
    fn screen_map_keeps_the_old_frame() {
        let mut f = PhoneFilter::new(0.0, 0.0, false);
        f.update([-1.0, 0.0, 0.0], [0.0; 3], 0.0);
        let (accel, gyro) = f.update([-0.95, 0.3, -0.05], [0.1, 0.2, 0.3], 0.01);
        assert!(close(&accel, &[0.3, -0.05, -0.95]));
        assert!(close(&gyro, &[0.2, 0.3, 0.1]));
    }

    #[test]
    fn phone_side_latch_survives_a_flat_phone() {
        let mut f = PhoneFilter::new(0.0, 0.0, true);
        f.update([-0.9, 0.0, -0.1], [0.0; 3], 0.0);
        // the phone is set down flat: gravity is on -Z and only noise is left on X
        for i in 1..10 {
            let noise = if i % 2 == 0 { 0.03 } else { -0.04 };
            f.update([noise, 0.5, -0.86], [0.0; 3], i as f32 * 0.01);
        }
        let (accel, _) = f.update([0.02, 0.5, -0.86], [0.0; 3], 1.0);
        assert!(accel[0] < -0.4, "the latched side flipped on noise, got {accel:?}");
    }

    #[test]
    fn phone_filter_smooths_the_tilt() {
        let mut f = PhoneFilter::new(0.05, 0.0, true);
        f.update([-1.0, 0.0, 0.0], [0.0; 3], 0.0);
        let rolled = [-0.95, 0.3, -0.05];
        let first = f.update(rolled, [0.0; 3], 0.01).0;
        assert!(first[0] < 0.0 && first[0] > -0.1, "first sample should be damped, got {first:?}");
        for i in 2..40 {
            f.update(rolled, [0.0; 3], i as f32 * 0.01);
        }
        let settled = f.update(rolled, [0.0; 3], 0.4).0;
        assert!(settled[0] < -0.28, "smoothing should converge on the tilt, got {settled:?}");
    }

    #[test]
    fn no_smoothing_passes_the_sample_through() {
        let mut f = PhoneFilter::new(0.0, 0.0, true);
        f.update([-1.0, 0.0, 0.0], [0.0; 3], 0.0);
        let (accel, gyro) = f.update([-0.95, 0.3, -0.05], [0.1, 0.2, 0.3], 0.01);
        assert!(close(&accel, &[-0.3, -0.95, -0.05]));
        assert!(close(&gyro, &[-0.2, 0.1, 0.3]));
    }

    #[test]
    fn matches_reference_fusion() {
        let mut f = Fusion::default();
        run(&mut f, 240, [0.0, 0.0, -1.0], [0.0; 3]);
        assert!(close(f.orientation().as_flattened(), &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]));

        run(&mut f, 120, [0.0, 0.0, -1.0], [0.25, 0.0, 0.0]);
        assert!(close(f.orientation().as_flattened(), &[1.0, 0.0, 0.0, 0.0, 0.301996, 0.953309, 0.0, -0.953309, 0.301996]));
        assert!(close(&f.rotation(), &[0.24999, 0.0, 0.0]));
        assert!(close(&f.gyro(), &[0.25, 0.0, 0.0]));

        run(&mut f, 1200, [-1.0, 0.0, 0.0], [0.0; 3]);
        let rolled = [-0.014437, 0.004343, 0.999887, 0.716666, 0.697379, 0.007319, -0.697268, 0.716690, -0.013181];
        assert!(close(f.orientation().as_flattened(), &rolled));
        assert!(close(&f.gyro(), &[0.0; 3]));

        run(&mut f, 60, [0.0, 0.6, -0.8], [0.0, -0.1, 0.05]);
        let mixed = [0.095546, 0.300315, 0.949043, 0.685034, 0.671894, -0.281581, -0.722219, 0.677030, -0.141530];
        assert!(close(f.orientation().as_flattened(), &mixed));
        assert!(close(&f.rotation(), &[0.24999, -0.049998, 0.024999]));
        assert!(close(&f.gyro(), &[0.0, -0.1, 0.05]));
    }
}
