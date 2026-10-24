//! Camera brand identification from EXIF Make/Model strings.

/// The brands this application knows how to frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Brand {
    Sony,
    Nikon,
    Canon,
    Leica,
    Hasselblad,
    PhaseOne,
    Alpa,
    Unknown,
}

impl Brand {
    /// Matches EXIF `Make` (primary signal) and `Model` (fallback for
    /// bodies that omit the make, like some Hasselblad firmware).
    pub fn resolve(make: &str, model: &str) -> Brand {
        let make = make.trim().to_uppercase();
        let model = model.trim().to_uppercase();

        // Checked before plain LEICA: these bodies ship with Phase One backs
        // and their owners identify them as Phase One systems.
        if make.starts_with("LEICA CAMERA GEOSYSTEMS") {
            return Brand::PhaseOne;
        }
        if make.starts_with("PHASE ONE") || make.starts_with("PHASEONE") {
            return Brand::PhaseOne;
        }
        // Phase One owns Mamiya and its backs report the Mamiya chassis.
        if make.starts_with("MAMIYA") || make == "MAM" {
            return Brand::PhaseOne;
        }
        if make.starts_with("SONY") {
            return Brand::Sony;
        }
        if make.starts_with("NIKON") {
            return Brand::Nikon;
        }
        if make.starts_with("CANON") {
            return Brand::Canon;
        }
        if make.starts_with("HASSELBLAD") {
            return Brand::Hasselblad;
        }
        if make.starts_with("LEICA") {
            return Brand::Leica;
        }
        if make.starts_with("ALPA") {
            return Brand::Alpa;
        }

        // Model fallback when Make is missing: Hasselblad V-system bodies
        // and CFV backs write distinctive model strings.
        if make.is_empty() && (model.starts_with("CFV") || model == "V" || model.starts_with("V "))
        {
            return Brand::Hasselblad;
        }

        Brand::Unknown
    }

    /// File stem of the brand's SVG assets (`{stem}.mono.svg`).
    pub fn asset_stem(self) -> &'static str {
        match self {
            Brand::Sony => "sony",
            Brand::Nikon => "nikon",
            Brand::Canon => "canon",
            Brand::Leica => "leica",
            Brand::Hasselblad => "hasselblad",
            Brand::PhaseOne => "phaseone",
            Brand::Alpa => "alpa",
            Brand::Unknown => "unknown",
        }
    }
}
