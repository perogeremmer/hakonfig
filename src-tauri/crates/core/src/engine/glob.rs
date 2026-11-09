//! Pencocokan pola model ala `enabledModels` di pi.
//!
//! pi (dan turunannya) memakai minimatch untuk pola seperti `groq/*` atau
//! `commandcode/**`. Pola itu dicocokkan ke `provider/modelId`, dan bedanya
//! penting: `*` **tidak** melewati garis miring, sementara `**` melewati.
//! Karena banyak provider punya id model bersarang (`commandcode/deepseek/…`),
//! pola `provider/*` diam-diam kehilangan model — sumber kebingungan yang mahal
//! kalau tidak dideteksi.

/// Cocokkan satu pola glob dengan teks, peka huruf besar/kecil seperti minimatch
/// dengan `nocase: true`.
pub fn matches(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let text: Vec<char> = text.to_lowercase().chars().collect();
    match_from(&pattern, 0, &text, 0)
}

fn match_from(pattern: &[char], mut p: usize, text: &[char], mut t: usize) -> bool {
    while p < pattern.len() {
        match pattern[p] {
            '*' => {
                // `**` melewati garis miring, `*` tidak.
                let double = pattern.get(p + 1) == Some(&'*');
                let next = if double { p + 2 } else { p + 1 };
                // `a/**/b` juga harus cocok dengan `a/b`, jadi `**/` boleh kosong.
                if double {
                    if let Some(after_slash) = skip_optional_slash(pattern, next) {
                        if match_from(pattern, after_slash, text, t) {
                            return true;
                        }
                    }
                }
                for consumed in t..=text.len() {
                    if consumed > t && !double && text[consumed - 1] == '/' {
                        break;
                    }
                    if match_from(pattern, next, text, consumed) {
                        return true;
                    }
                }
                return false;
            }
            '?' => {
                if t >= text.len() || text[t] == '/' {
                    return false;
                }
                p += 1;
                t += 1;
            }
            literal => {
                if t >= text.len() || text[t] != literal {
                    return false;
                }
                p += 1;
                t += 1;
            }
        }
    }
    t == text.len()
}

/// Untuk pola berbentuk `**/`, index setelah garis miring — dipakai supaya
/// `provider/**` juga cocok dengan referensi tanpa segmen tambahan.
fn skip_optional_slash(pattern: &[char], index: usize) -> Option<usize> {
    if pattern.get(index) == Some(&'/') {
        Some(index + 1)
    } else {
        None
    }
}

/// Apakah salah satu pola menutupi referensi `provider/model`.
pub fn covers<S: AsRef<str>>(patterns: &[S], reference: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| matches(pattern.as_ref(), reference))
}

/// Pola yang paling lebar untuk satu provider: `<id>/**`.
pub fn provider_wide_pattern(provider: &str) -> String {
    format!("{}/**", provider)
}

/// Pola yang tampak mencakup provider tetapi berhenti di satu segmen.
pub fn provider_narrow_pattern(provider: &str) -> String {
    format!("{}/*", provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_star_does_not_cross_a_slash() {
        assert!(matches("groq/*", "groq/llama-3.3-70b"));
        assert!(!matches("groq/*", "groq/deepseek/deepseek-v4-pro"));
        assert!(matches("groq/**", "groq/deepseek/deepseek-v4-pro"));
    }

    #[test]
    fn double_star_matches_zero_segments() {
        assert!(matches("groq/**", "groq/llama"));
        assert!(matches("**", "a/b/c"));
        assert!(matches("*", "a"));
        assert!(!matches("*", "a/b"));
    }

    #[test]
    fn question_mark_is_one_character_without_slash() {
        assert!(matches("groq/llama-?.x", "groq/llama-a.x"));
        assert!(!matches("groq/?", "groq/a/b"));
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert!(matches("GROQ/*", "groq/Llama"));
    }

    #[test]
    fn covers_reports_any_matching_pattern() {
        let patterns = vec!["deepseek/*".to_string(), "commandcode/**".to_string()];
        assert!(covers(&patterns, "deepseek/chat"));
        assert!(covers(&patterns, "commandcode/deepseek/x"));
        assert!(!covers(&patterns, "router-perogeremmer/x"));
    }

    #[test]
    fn provider_patterns_are_built_from_the_id() {
        assert_eq!(provider_wide_pattern("groq"), "groq/**");
        assert_eq!(provider_narrow_pattern("groq"), "groq/*");
    }
}
