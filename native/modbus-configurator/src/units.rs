//! Display conversions only. Register values, TSVs and history retain native units.
// Each entry is (display unit, multiplier, offset) relative to the native value.
pub fn choices(native: &str) -> &'static [(&'static str, f64, f64)] {
    match native {
        "°C" => &[("°C", 1.0, 0.0), ("°F", 1.8, 32.0), ("K", 1.0, 273.15)],
        "°F" => &[
            ("°F", 1.0, 0.0),
            ("°C", 5.0 / 9.0, -160.0 / 9.0),
            ("K", 5.0 / 9.0, 255.3722222222222),
        ],
        "gr/ft³" => &[("gr/ft³", 1.0, 0.0), ("g/m³", 2.28835191, 0.0)],
        "gr/lb" => &[("gr/lb", 1.0, 0.0), ("g/kg", 1.0 / 7.0, 0.0)],
        "Btu/lb" => &[("Btu/lb", 1.0, 0.0), ("kJ/kg", 2.326, 0.0)],
        "bara" => &[
            ("bara", 1.0, 0.0),
            ("kPa(a)", 100.0, 0.0),
            ("MPa(a)", 0.1, 0.0),
            ("psia", 14.503773773, 0.0),
        ],
        "bar" => &[
            ("bar", 1.0, 0.0),
            ("kPa", 100.0, 0.0),
            ("MPa", 0.1, 0.0),
            ("psi", 14.503773773, 0.0),
        ],
        "ppmv" => &[("ppmv", 1.0, 0.0), ("% vol", 0.0001, 0.0)],
        "g/m³" => &[("g/m³", 1.0, 0.0), ("mg/m³", 1000.0, 0.0)],
        "g/kg" => &[("g/kg", 1.0, 0.0), ("mg/kg", 1000.0, 0.0)],
        "kJ/kg" => &[
            ("kJ/kg", 1.0, 0.0),
            ("J/kg", 1000.0, 0.0),
            ("Btu/lb", 0.4299226139294927, 0.0),
        ],
        "W" => &[("W", 1.0, 0.0), ("kW", 0.001, 0.0)],
        "kWh" => &[("kWh", 1.0, 0.0), ("Wh", 1000.0, 0.0)],
        "V" => &[("V", 1.0, 0.0), ("kV", 0.001, 0.0)],
        "Vac" => &[("Vac", 1.0, 0.0), ("kVac", 0.001, 0.0)],
        "A" => &[("A", 1.0, 0.0), ("mA", 1000.0, 0.0)],
        "Hz" => &[("Hz", 1.0, 0.0), ("kHz", 0.001, 0.0)],
        _ => &[],
    }
}

#[derive(Clone, Copy, Default, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Preset {
    System,
    Us,
    #[default]
    Uk,
    Eu,
}
impl Preset {
    pub fn resolve(self, region: Option<&str>) -> Self {
        if self != Self::System {
            return self;
        }
        match region {
            Some("US") => Self::Us,
            Some("GB") => Self::Uk,
            _ => Self::Eu,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Us => "US",
            Self::Uk => "UK",
            Self::Eu => "EU",
        }
    }
    pub fn index(self, native: &str) -> usize {
        match (self, native) {
            (Self::Us, "°C") => 1,
            (Self::Uk | Self::Eu, "°F" | "gr/ft³" | "gr/lb" | "Btu/lb") => 1,
            (Self::Us, "bara" | "bar") => 3,
            (Self::Us, "kJ/kg") => 2,
            (Self::Eu, "bara" | "bar") => 1,
            _ => 0,
        }
    }
}
pub fn system_region() -> Option<String> {
    #[cfg(windows)]
    {
        let mut buffer = [0u16; 85];
        // Windows writes at most the supplied UTF-16 capacity, including its terminator.
        let length = unsafe {
            windows_sys::Win32::Globalization::GetUserDefaultGeoName(
                buffer.as_mut_ptr(),
                buffer.len() as i32,
            )
        };
        if length > 1 && length as usize <= buffer.len() {
            return String::from_utf16(&buffer[..length as usize - 1]).ok();
        }
    }
    None
}

#[cfg(test)]
mod native_bank_tests {
    use super::*;
    #[test]
    fn nonmetric_native_values_convert_without_changing_source() {
        for (unit, native, expected) in [
            ("°F", 68.0, 20.0),
            ("gr/lb", 7.0, 1.0),
            ("gr/ft³", 1.0, 2.28835191),
            ("Btu/lb", 1.0, 2.326),
        ] {
            let (_, scale, offset) = choices(unit)[Preset::Eu.index(unit)];
            assert!((native * scale + offset - expected).abs() < 1e-8);
            assert_eq!(Preset::Us.index(unit), 0);
        }
    }
}
