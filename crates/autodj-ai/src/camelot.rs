//! Camelot Wheel distance function for harmonic mixing.

const CAMELOT_KEYS: [&str; 24] = [
    "1A", "1B", "2A", "2B", "3A", "3B", "4A", "4B", "5A", "5B", "6A", "6B",
    "7A", "7B", "8A", "8B", "9A", "9B", "10A", "10B", "11A", "11B", "12A", "12B",
];

/// Computes the harmonic distance between two Camelot keys.
/// Returns 0 for an exact match, 1 for adjacent, 2 for related, etc.
pub fn camelot_distance(key_a: &str, key_b: &str) -> u32 {
    let a_idx = parse_camelot(key_a);
    let b_idx = parse_camelot(key_b);

    match (a_idx, b_idx) {
        (Some((num_a, is_major_a)), Some((num_b, is_major_b))) => {
            let mut diff = ((num_a as i32 - num_b as i32).abs()) as u32;
            if diff > 6 {
                diff = 12 - diff; // Wrap around the clock
            }

            if is_major_a == is_major_b {
                // Same mode (e.g., A to A)
                diff
            } else {
                // Different mode (e.g., A to B)
                // If diff is 0, they are relative major/minor (e.g., 8A <-> 8B), which is distance 1.
                if diff == 0 {
                    1
                } else {
                    diff + 1
                }
            }
        }
        _ => 12, // Maximum distance if parsing fails
    }
}

/// Parses a Camelot key string (e.g., "8A") into (number, is_major).
/// Major = B, Minor = A based on typical Camelot notation.
fn parse_camelot(key: &str) -> Option<(u8, bool)> {
    let key = key.trim().to_uppercase();
    if key.is_empty() {
        return None;
    }

    let is_major = key.ends_with('B');
    let num_str = key.trim_end_matches(|c| c == 'A' || c == 'B');
    num_str.parse::<u8>().ok().filter(|&n| n >= 1 && n <= 12).map(|n| (n, is_major))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camelot_distance() {
        assert_eq!(camelot_distance("8A", "8A"), 0);
        assert_eq!(camelot_distance("8A", "9A"), 1); // +1 hour
        assert_eq!(camelot_distance("8A", "7A"), 1); // -1 hour
        assert_eq!(camelot_distance("12A", "1A"), 1); // Wrap around
        assert_eq!(camelot_distance("8A", "8B"), 1); // Mode change
        assert_eq!(camelot_distance("8A", "2A"), 6); // Opposite
        assert_eq!(camelot_distance("8A", "9B"), 2); // +1 hour AND mode change
    }
}
