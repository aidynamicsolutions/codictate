/**
 * Whisper-supported languages with emoji flags for display.
 * This file contains all 100 languages supported by OpenAI Whisper.
 */

/**
 * Union type of all valid Whisper language codes for type safety.
 */
export type WhisperLanguageCode =
  | "af" | "am" | "ar" | "hy" | "as" | "az" | "ba" | "eu" | "be" | "bn"
  | "bs" | "br" | "bg" | "my" | "ca" | "yue" | "zh" | "hr" | "cs" | "da"
  | "nl" | "en" | "et" | "fo" | "fi" | "fr" | "gl" | "ka" | "de" | "el"
  | "gu" | "ht" | "ha" | "haw" | "he" | "hi" | "hu" | "is" | "id" | "it"
  | "ja" | "jw" | "kn" | "kk" | "km" | "ko" | "lo" | "la" | "lv" | "ln"
  | "lt" | "lb" | "mk" | "mg" | "ms" | "ml" | "mt" | "mi" | "mr" | "mn"
  | "ne" | "no" | "nn" | "oc" | "ps" | "fa" | "pl" | "pt" | "pa" | "ro"
  | "ru" | "sa" | "sr" | "sn" | "sd" | "si" | "sk" | "sl" | "so" | "es"
  | "su" | "sw" | "sv" | "tl" | "tg" | "ta" | "tt" | "te" | "th" | "bo"
  | "tr" | "tk" | "uk" | "ur" | "uz" | "vi" | "cy" | "yi" | "yo" | "zu";

export interface WhisperLanguage {
  code: WhisperLanguageCode;
  label: string;
  flag: string;
}

/**
 * Complete list of Whisper-supported languages.
 * Sorted alphabetically by English label.
 */
export const WHISPER_LANGUAGES: WhisperLanguage[] = [
  { code: "af", label: "Afrikaans", flag: "🇿🇦" },
  { code: "am", label: "Amharic", flag: "🇪🇹" },
  { code: "ar", label: "Arabic", flag: "🇸🇦" },
  { code: "hy", label: "Armenian", flag: "🇦🇲" },
  { code: "as", label: "Assamese", flag: "🇮🇳" },
  { code: "az", label: "Azerbaijani", flag: "🇦🇿" },
  { code: "ba", label: "Bashkir", flag: "🇷🇺" },
  { code: "eu", label: "Basque", flag: "🇪🇸" },
  { code: "be", label: "Belarusian", flag: "🇧🇾" },
  { code: "bn", label: "Bengali", flag: "🇧🇩" },
  { code: "bs", label: "Bosnian", flag: "🇧🇦" },
  { code: "br", label: "Breton", flag: "🇫🇷" },
  { code: "bg", label: "Bulgarian", flag: "🇧🇬" },
  { code: "my", label: "Burmese", flag: "🇲🇲" },
  { code: "ca", label: "Catalan", flag: "🇪🇸" },
  { code: "yue", label: "Cantonese", flag: "🇭🇰" },
  { code: "zh", label: "Chinese", flag: "🇨🇳" },
  { code: "hr", label: "Croatian", flag: "🇭🇷" },
  { code: "cs", label: "Czech", flag: "🇨🇿" },
  { code: "da", label: "Danish", flag: "🇩🇰" },
  { code: "nl", label: "Dutch", flag: "🇳🇱" },
  { code: "en", label: "English", flag: "🇺🇸" },
  { code: "et", label: "Estonian", flag: "🇪🇪" },
  { code: "fo", label: "Faroese", flag: "🇫🇴" },
  { code: "fi", label: "Finnish", flag: "🇫🇮" },
  { code: "fr", label: "French", flag: "🇫🇷" },
  { code: "gl", label: "Galician", flag: "🇪🇸" },
  { code: "ka", label: "Georgian", flag: "🇬🇪" },
  { code: "de", label: "German", flag: "🇩🇪" },
  { code: "el", label: "Greek", flag: "🇬🇷" },
  { code: "gu", label: "Gujarati", flag: "🇮🇳" },
  { code: "ht", label: "Haitian Creole", flag: "🇭🇹" },
  { code: "ha", label: "Hausa", flag: "🇳🇬" },
  { code: "haw", label: "Hawaiian", flag: "🇺🇸" },
  { code: "he", label: "Hebrew", flag: "🇮🇱" },
  { code: "hi", label: "Hindi", flag: "🇮🇳" },
  { code: "hu", label: "Hungarian", flag: "🇭🇺" },
  { code: "is", label: "Icelandic", flag: "🇮🇸" },
  { code: "id", label: "Indonesian", flag: "🇮🇩" },
  { code: "it", label: "Italian", flag: "🇮🇹" },
  { code: "ja", label: "Japanese", flag: "🇯🇵" },
  { code: "jw", label: "Javanese", flag: "🇮🇩" },
  { code: "kn", label: "Kannada", flag: "🇮🇳" },
  { code: "kk", label: "Kazakh", flag: "🇰🇿" },
  { code: "km", label: "Khmer", flag: "🇰🇭" },
  { code: "ko", label: "Korean", flag: "🇰🇷" },
  { code: "lo", label: "Lao", flag: "🇱🇦" },
  { code: "la", label: "Latin", flag: "🇻🇦" },
  { code: "lv", label: "Latvian", flag: "🇱🇻" },
  { code: "ln", label: "Lingala", flag: "🇨🇩" },
  { code: "lt", label: "Lithuanian", flag: "🇱🇹" },
  { code: "lb", label: "Luxembourgish", flag: "🇱🇺" },
  { code: "mk", label: "Macedonian", flag: "🇲🇰" },
  { code: "mg", label: "Malagasy", flag: "🇲🇬" },
  { code: "ms", label: "Malay", flag: "🇲🇾" },
  { code: "ml", label: "Malayalam", flag: "🇮🇳" },
  { code: "mt", label: "Maltese", flag: "🇲🇹" },
  { code: "mi", label: "Maori", flag: "🇳🇿" },
  { code: "mr", label: "Marathi", flag: "🇮🇳" },
  { code: "mn", label: "Mongolian", flag: "🇲🇳" },
  { code: "ne", label: "Nepali", flag: "🇳🇵" },
  { code: "no", label: "Norwegian", flag: "🇳🇴" },
  { code: "nn", label: "Nynorsk", flag: "🇳🇴" },
  { code: "oc", label: "Occitan", flag: "🇫🇷" },
  { code: "ps", label: "Pashto", flag: "🇦🇫" },
  { code: "fa", label: "Persian", flag: "🇮🇷" },
  { code: "pl", label: "Polish", flag: "🇵🇱" },
  { code: "pt", label: "Portuguese", flag: "🇵🇹" },
  { code: "pa", label: "Punjabi", flag: "🇮🇳" },
  { code: "ro", label: "Romanian", flag: "🇷🇴" },
  { code: "ru", label: "Russian", flag: "🇷🇺" },
  { code: "sa", label: "Sanskrit", flag: "🇮🇳" },
  { code: "sr", label: "Serbian", flag: "🇷🇸" },
  { code: "sn", label: "Shona", flag: "🇿🇼" },
  { code: "sd", label: "Sindhi", flag: "🇵🇰" },
  { code: "si", label: "Sinhala", flag: "🇱🇰" },
  { code: "sk", label: "Slovak", flag: "🇸🇰" },
  { code: "sl", label: "Slovenian", flag: "🇸🇮" },
  { code: "so", label: "Somali", flag: "🇸🇴" },
  { code: "es", label: "Spanish", flag: "🇪🇸" },
  { code: "su", label: "Sundanese", flag: "🇮🇩" },
  { code: "sw", label: "Swahili", flag: "🇰🇪" },
  { code: "sv", label: "Swedish", flag: "🇸🇪" },
  { code: "tl", label: "Tagalog", flag: "🇵🇭" },
  { code: "tg", label: "Tajik", flag: "🇹🇯" },
  { code: "ta", label: "Tamil", flag: "🇮🇳" },
  { code: "tt", label: "Tatar", flag: "🇷🇺" },
  { code: "te", label: "Telugu", flag: "🇮🇳" },
  { code: "th", label: "Thai", flag: "🇹🇭" },
  { code: "bo", label: "Tibetan", flag: "🇨🇳" },
  { code: "tr", label: "Turkish", flag: "🇹🇷" },
  { code: "tk", label: "Turkmen", flag: "🇹🇲" },
  { code: "uk", label: "Ukrainian", flag: "🇺🇦" },
  { code: "ur", label: "Urdu", flag: "🇵🇰" },
  { code: "uz", label: "Uzbek", flag: "🇺🇿" },
  { code: "vi", label: "Vietnamese", flag: "🇻🇳" },
  { code: "cy", label: "Welsh", flag: "🏴󠁧󠁢󠁷󠁬󠁳󠁿" },
  { code: "yi", label: "Yiddish", flag: "🇮🇱" },
  { code: "yo", label: "Yoruba", flag: "🇳🇬" },
  { code: "zu", label: "Zulu", flag: "🇿🇦" },
];

/**
 * Get a language by its code.
 */
export const getLanguageByCode = (code: string): WhisperLanguage | undefined => {
  return WHISPER_LANGUAGES.find((lang) => lang.code === code);
};

/**
 * Get the emoji flag for a language code.
 * Returns a globe emoji for "auto" or unknown codes.
 */
export const getLanguageFlag = (code: string): string => {
  if (code === "auto") return "🌐";
  return getLanguageByCode(code)?.flag ?? "🌐";
};

/**
 * Get the display label for a language code.
 * For "auto", returns undefined so the caller can use i18n.
 */
export const getLanguageLabel = (code: string): string | undefined => {
  if (code === "auto") return undefined;
  return getLanguageByCode(code)?.label;
};

/**
 * Total count of supported languages (excluding auto-detect).
 */
export const WHISPER_LANGUAGE_COUNT = WHISPER_LANGUAGES.length;
