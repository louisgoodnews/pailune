//! The window icon, drawn in code (a crescent moon, like the logo).

use eframe::egui::IconData;

const SIZE: u32 = 64;
const ACCENT: [u8; 4] = [0xd0, 0x8a, 0x3c, 0xff];
const BACKGROUND: [u8; 4] = [0x17, 0x19, 0x1d, 0xff];

/// 64×64 RGBA crescent matching `packaging/pylune.svg`.
pub fn icon() -> IconData {
    let inside =
        |x: f32, y: f32, cx: f32, cy: f32, r: f32| (x - cx).powi(2) + (y - cy).powi(2) <= r * r;
    let rgba = (0..SIZE * SIZE)
        .flat_map(|i| {
            let (x, y) = ((i % SIZE) as f32 + 0.5, (i / SIZE) as f32 + 0.5);
            match (
                inside(x, y, 32.0, 32.0, 20.0),
                inside(x, y, 41.0, 26.0, 17.0),
            ) {
                (true, false) => ACCENT,
                _ => BACKGROUND,
            }
        })
        .collect();
    IconData {
        rgba,
        width: SIZE,
        height: SIZE,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn draws_a_crescent() {
        let icon = super::icon();
        assert_eq!(icon.rgba.len(), 64 * 64 * 4);
        let accent = icon.rgba.chunks(4).filter(|p| p == &super::ACCENT).count();
        assert!((300..1200).contains(&accent), "{accent} crescent pixels");
    }
}
