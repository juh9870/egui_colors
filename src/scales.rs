#![allow(clippy::cast_lossless)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::suboptimal_flops)]

use crate::color_space::{from_degrees, LinSrgb, Okhsl};
use crate::{apca::estimate_lc, tokens::ThemeColor};
use egui::{epaint::Hsva, Color32};

#[derive(Debug, Default, Clone)]
pub struct Scales {
    pub custom: Hsva,
    pub okhsl: [Okhsl; 12],
    pub rgbs: [LinSrgb; 12],
    pub srgb: LinSrgb,
    pub scale: [Color32; 12],
    pub dark_mode: bool,
}

impl Scales {
    pub fn custom(&self) -> [u8; 3] {
        self.custom.to_srgb()
    }

    pub fn process_color(&mut self, v: ThemeColor) {
        self.srgb = v.get_srgb();
        self.draw_scale();
    }

    fn draw_scale(&mut self) {
        if self.dark_mode {
            self.dark_scale();
        } else {
            self.light_scale();
        }
    }

    pub fn clamp_custom(&mut self) {
        // --------
        // ---- input value in color picker clamped to useable values---
        // ----------------------------------------------
        let v_clamp = match self.custom.s {
            (0.0..=0.3) => ((0.0 - 0.13) / (0.3 - 0.0) as f32).mul_add(self.custom.s - 0.0, 0.13),
            (0.3..=1.0) => ((0.13 - 0.0) / (1.0 - 0.3) as f32).mul_add(self.custom.s - 0.3, 0.0),
            _ => 0.,
        };
        let s_clamp = match self.custom.v {
            (0.0..=0.13) => ((0. - 0.3) / (0.13 - 0.0) as f32).mul_add(self.custom.v - 0.0, 0.3),
            (0.13..=1.0) => ((0.3 - 0.) / (1. - 0.13) as f32).mul_add(self.custom.v - 0.13, 0.),
            _ => 0.,
        };
        self.custom.v = self.custom.v.clamp(v_clamp, 0.99);
        self.custom.s = self.custom.s.clamp(s_clamp, 1.0);
    }

    fn light_scale(&mut self) {
        self.rgbs[8] = self.srgb;
        let hsl = Okhsl::from_color(self.srgb);
        let hue = hsl.as_degrees();
        self.okhsl[8] = hsl;
        let srgb = self.srgb;

        let lighten_values = [0.965, 0.9, 0.82, 0.75, 0.63, 0.51, 0.39, 0.27];
        let clamp_v = [0.99, 0.98, 0.97, 0.95, 0.93, 0.90, 0.88, 0.85];
        let darken_values = [0.1, 0.2, 0.55];
        for (i, v) in lighten_values.iter().enumerate() {
            self.rgbs[i] = srgb.lighten(*v);
        }

        for i in 0..12 {
            if (0..9).contains(&i) {
                self.okhsl[i] = Okhsl::from_color(self.rgbs[i]);
                if i != 8 {
                    // adapt hue to compensate for temperature shift
                    if hue > 0. && hue < 90. {
                        self.okhsl[i].hue =
                            from_degrees(self.okhsl[i].as_degrees() + 10_f32 - i as f32);
                    }
                    if hue > 200. && hue < 280. {
                        self.okhsl[i].hue =
                            from_degrees(self.okhsl[i].as_degrees() - 10_f32 - i as f32);
                    }
                }
            }
            if (9..12).contains(&i) {
                self.okhsl[i] = Okhsl::from_color(srgb).darken(darken_values[i - 9]);
            }
            if i != 8 {
                // enhance saturation for all values (except orginal) and diminish for certain hues (greenish)
                let hue = hue as u8;
                let sat_val = match hue {
                    159..=216 => ((hue - 159) as f32 / 58_f32) * 0.25,
                    100..=158 => ((158 - hue) as f32 / 58_f32) * 0.25,
                    _ => 0.25,
                };
                let sat_clamp = match hue {
                    100..=158 => ((hue - 100) as f32 / 58_f32) * 0.12,
                    159..=217 => ((217 - hue) as f32 / 58_f32) * 0.12,
                    _ => 0.0,
                };
                if hsl.saturation > 0.01 && hsl.lightness > 0.01 {
                    self.okhsl[i].saturation =
                        (hsl.saturation * hsl.lightness + sat_val).clamp(0.1, 1.0 - sat_clamp);
                }
                if i < 8 && hsl.lightness > 0.79 {
                    self.okhsl[i].lightness =
                        self.okhsl[i].lightness.clamp(clamp_v[i] - 0.8, clamp_v[i]);
                }
            }
        }
        self.okhsl[10].lightness = self.okhsl[10].lightness.clamp(0.43, 0.50);
        self.okhsl[11].lightness *= 0.9;

        let [x, y, z] = hsl.to_u8();
        let lc = estimate_lc(Color32::WHITE, Color32::from_rgb(x, y, z));
        if lc > -46. {
            self.okhsl[8].lightness = 0.68;
            self.okhsl[9].lightness = self.okhsl[8].lightness * 0.9;
            self.okhsl[9].saturation = self.okhsl[8].saturation * 0.9;
            // }
        } else {
            self.okhsl[9].saturation = self.okhsl[8].saturation;
        }

        for i in 0..12 {
            let [r, g, b]: [u8; 3] = self.okhsl[i].to_u8();
            self.scale[i] = Color32::from_rgb(r, g, b);
        }
    }

    pub fn dark_scale(&mut self) {
        self.rgbs[8] = self.srgb;
        let hsl = Okhsl::from_color(self.srgb);
        let hue = hsl.as_degrees();
        self.okhsl[8] = hsl;

        let darken_values = [0.975, 0.96, 0.93, 0.89, 0.83, 0.75, 0.64, 0.39];
        let clamp_s = [0.3, 0.5, 0.8, 1., 1., 0.95, 0.7, 0.8];
        let clamp_s2 = [0.14, 0.16, 0.44, 0.62, 0.61, 0.56, 0.52, 0.51];
        let clamp_l = [0.08, 0.10, 0.15, 0.19, 0.23, 0.29, 0.36, 0.47];
        let lighten_values = [0.095, 0.45, 0.75];

        for i in 0..8 {
            self.rgbs[i] = self.srgb.darken(darken_values[i]);
            self.okhsl[i] = Okhsl::from_color(self.rgbs[i]);
            if (259.0..=323.).contains(&hue) {
                self.okhsl[i] = self.okhsl[i].lighten((i + 1) as f32 * 0.011);
            }
            if (323.0..=350.).contains(&hue) && i == (6 | 7) {
                self.okhsl[i] = self.okhsl[i].lighten((i + 1) as f32 * 0.01);
            }
            self.okhsl[i].saturation *= 1.0 + ((1.0 - hsl.saturation) * 2.);

            if hsl.saturation > 0.36 {
                self.okhsl[i].saturation = self.okhsl[i].saturation.clamp(
                    clamp_s2[i],
                    (hsl.saturation * clamp_s[i]).clamp(clamp_s2[i] + 0.01, 1.0),
                );
            } else {
                self.okhsl[i].saturation = self.okhsl[i]
                    .saturation
                    .clamp(0.0, hsl.saturation * clamp_s[i]);
            }
            self.okhsl[i].lightness = self.okhsl[i].lightness.clamp(
                clamp_l[i],
                (clamp_l[i] * (1.71 - hsl.saturation)).clamp(clamp_l[i] + 0.01, 1.0),
            );
        }
        for i in 9..12 {
            self.okhsl[i] = hsl.lighten(lighten_values[i - 9]);
            if (0.0..=90.).contains(&hue) | (300.0..=350.).contains(&hue) {
                self.okhsl[i].hue =
                    from_degrees(self.okhsl[i].as_degrees() + 2_f32 * (i - 8) as f32);
            }
            if (100.0..=280.).contains(&hue) {
                self.okhsl[i].hue =
                    from_degrees(self.okhsl[i].as_degrees() - 2_f32 * (i - 8) as f32);
            }
        }
        self.okhsl[10].lightness = self.okhsl[10].lightness.clamp(0.73, 1.0);
        self.okhsl[11].lightness = self.okhsl[11].lightness.clamp(0.88, 1.0);
        if (115.0..=220.).contains(&hue) {
            self.okhsl[11].saturation = self.okhsl[11].saturation.clamp(0.0, hsl.saturation * 0.75);
            self.okhsl[10].saturation = self.okhsl[10].saturation.clamp(0.0, hsl.saturation * 0.9);
        }
        let [x, y, z] = hsl.to_u8();
        let lc = estimate_lc(Color32::WHITE, Color32::from_rgb(x, y, z));
        if lc < -95.4 {
            self.okhsl[8] = hsl.lighten(0.3);
            self.okhsl[8].saturation = (hsl.saturation * 1.25).clamp(0., 1.);
            self.okhsl[9] = self.okhsl[9].lighten(0.25);
            self.okhsl[9].saturation = hsl.saturation;
        }
        (0..12).for_each(|i| {
            let [r, g, b]: [u8; 3] = self.okhsl[i].to_u8();
            self.scale[i] = Color32::from_rgb(r, g, b);
        });
    }
}
