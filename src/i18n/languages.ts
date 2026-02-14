/**
 * Language metadata for supported locales.
 *
 * To add a new language:
 * 1. Create a new folder: src/i18n/locales/{code}/translation.json
 * 2. Add an entry here with the language code, English name, and native name
 * 3. Optionally add a priority (lower = higher in dropdown, no priority = alphabetical at end)
 * 4. For RTL languages, add direction: 'rtl'
 */
export const LANGUAGE_METADATA: Record<
  string,
  {
    name: string;
    nativeName: string;
    flag: string;
    priority?: number;
    direction?: "ltr" | "rtl";
  }
> = {
  en: { name: "English", nativeName: "English", flag: "🇺🇸", priority: 1 },
  zh: { name: "Simplified Chinese", nativeName: "简体中文", flag: "🇨🇳", priority: 2 },
  "zh-TW": { name: "Traditional Chinese", nativeName: "繁體中文", flag: "🇹🇼", priority: 3 },
  es: { name: "Spanish", nativeName: "Español", flag: "🇪🇸", priority: 4 },
  fr: { name: "French", nativeName: "Français", flag: "🇫🇷", priority: 5 },
  de: { name: "German", nativeName: "Deutsch", flag: "🇩🇪", priority: 6 },
  ja: { name: "Japanese", nativeName: "日本語", flag: "🇯🇵", priority: 7 },
  ko: { name: "Korean", nativeName: "한국어", flag: "🇰🇷", priority: 8 },
  vi: { name: "Vietnamese", nativeName: "Tiếng Việt", flag: "🇻🇳", priority: 9 },
  pl: { name: "Polish", nativeName: "Polski", flag: "🇵🇱", priority: 10 },
  it: { name: "Italian", nativeName: "Italiano", flag: "🇮🇹", priority: 11 },
  ru: { name: "Russian", nativeName: "Русский", flag: "🇷🇺", priority: 12 },
  uk: { name: "Ukrainian", nativeName: "Українська", flag: "🇺🇦", priority: 13 },
  pt: { name: "Portuguese", nativeName: "Português", flag: "🇧🇷", priority: 14 },
  cs: { name: "Czech", nativeName: "Čeština", flag: "🇨🇿", priority: 15 },
  tr: { name: "Turkish", nativeName: "Türkçe", flag: "🇹🇷", priority: 16 },
  ar: { name: "Arabic", nativeName: "العربية", flag: "🇸🇦", priority: 17, direction: "rtl" },
};
