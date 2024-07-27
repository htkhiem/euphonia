use gtk::gio;
use gtk::Ordering;
use crate::config::APPLICATION_ID;
use mpd::status::AudioFormat;

pub fn settings_manager() -> gio::Settings {
    // Trim the .Devel suffix if exists
    let app_id = APPLICATION_ID.trim_end_matches(".Devel");
    gio::Settings::new(app_id)
}

pub fn format_secs_as_duration(seconds: f64) -> String {
    let total_seconds = seconds.round() as i64;
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if days > 0 {
        format!(
            "{} days {:02}:{:02}:{:02}",
            days, hours, minutes, seconds
        )
    } else if hours > 0 {
        format!(
            "{:02}:{:02}:{:02}",
            hours, minutes, seconds
        )
    } else {
        format!(
            "{:02}:{:02}",
            minutes, seconds
        )
    }
}

// For convenience
pub fn prettify_audio_format(format: &AudioFormat) -> String {
    // Here we need to re-infer whether this format is DSD or PCM
    // Only detect DSD64 at minimum, anything lower is too esoteric
    if format.bits == 1 && format.rate >= 352800 {
        // Is probably DSD
        let sample_rate = format.rate * 8;
        return format!(
            "{} ({:.4}MHz) {}ch",
            sample_rate / 44100,
            (sample_rate as f64) / 1e6,
            format.chans
        );
    }
    format!(
        "{}bit {:.1}kHz {}ch",
        format.bits,
        (format.rate as f64) / 1e3,
        format.chans
    )
}

pub fn g_cmp_options<T: Ord>(s1: Option<&T>, s2: Option<&T>, nulls_first: bool, asc: bool) -> Ordering {
    if s1.is_none() && s2.is_none() {
        return Ordering::Equal;
    }
    else if s1.is_none() {
        if nulls_first {
            return Ordering::Larger;
        }
        return Ordering::Smaller;
    }
    else if s2.is_none() {
        if nulls_first {
            return Ordering::Smaller;
        }
        return Ordering::Larger;
    }
    if asc {
        return Ordering::from(s1.unwrap().cmp(s2.unwrap()));
    }
    Ordering::from(s2.unwrap().cmp(s1.unwrap()))
}

pub fn g_cmp_str_options(
    s1: Option<&str>, s2: Option<&str>,
    nulls_first: bool, asc: bool,
    case_sensitive: bool
) -> Ordering {
    if s1.is_none() && s2.is_none() {
        return Ordering::Equal;
    }
    else if s1.is_none() {
        if nulls_first {
            return Ordering::Larger;
        }
        return Ordering::Smaller;
    }
    else if s2.is_none() {
        if nulls_first {
            return Ordering::Smaller;
        }
        return Ordering::Larger;
    }
    if asc {
        if case_sensitive {
            return Ordering::from(s1.unwrap().cmp(s2.unwrap()));
        }
        return Ordering::from(
            s1.unwrap().to_lowercase().cmp(
                &s2.unwrap().to_lowercase()
            )
        );
    }
    if case_sensitive {
        return Ordering::from(s2.unwrap().cmp(s1.unwrap()));
    }
    Ordering::from(
        s2.unwrap().to_lowercase().cmp(
            &s1.unwrap().to_lowercase()
        )
    )
}
