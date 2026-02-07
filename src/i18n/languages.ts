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
  zh: { name: "Chinese", nativeName: "中文", flag: "🇨🇳", priority: 2 },
  es: { name: "Spanish", nativeName: "Español", flag: "🇪🇸", priority: 3 },
  fr: { name: "French", nativeName: "Français", flag: "🇫🇷", priority: 4 },
  de: { name: "German", nativeName: "Deutsch", flag: "🇩🇪", priority: 5 },
  ja: { name: "Japanese", nativeName: "日本語", flag: "🇯🇵", priority: 6 },
  ko: { name: "Korean", nativeName: "한국어", flag: "🇰🇷", priority: 7 },
  vi: { name: "Vietnamese", nativeName: "Tiếng Việt", flag: "🇻🇳", priority: 8 },
  pl: { name: "Polish", nativeName: "Polski", flag: "🇵🇱", priority: 9 },
  it: { name: "Italian", nativeName: "Italiano", flag: "🇮🇹", priority: 10 },
  ru: { name: "Russian", nativeName: "Русский", flag: "🇷🇺", priority: 11 },
  uk: { name: "Ukrainian", nativeName: "Українська", flag: "🇺🇦", priority: 12 },
  pt: { name: "Portuguese", nativeName: "Português", flag: "🇧🇷", priority: 13 },
  cs: { name: "Czech", nativeName: "Čeština", flag: "🇨🇿", priority: 14 },
  tr: { name: "Turkish", nativeName: "Türkçe", flag: "🇹🇷", priority: 15 },
  ar: { name: "Arabic", nativeName: "العربية", flag: "🇸🇦", priority: 16, direction: "rtl" },
};
