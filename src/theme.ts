/**
 * Centralized theme configuration for Handy.
 *
 * This file contains all brand colors and theming constants.
 * To rebrand the app, update the colors here and in App.css.
 *
 * @see doc/branding.md for complete rebranding instructions.
 */

/**
 * Brand color palette.
 * These are the hex values used throughout the app.
 */
export const colors = {
  /** Main brand color (coral red) - used for primary UI elements */
  primary: "#e63946",

  /** Dark mode variant of primary color */
  primaryDark: "#f07178",

  /** UI background accent color */
  uiBackground: "#c1121f",

  /** Logo stroke color (light mode) */
  stroke: "#2b1d1e",

  /** Logo stroke color (dark mode) */
  strokeDark: "#ffd5d5",

  /** Secondary highlight color */
  highlight: "#ffb3b3",

  /** Recording overlay bar background */
  bar: "#ffe0e0",

  /** Recording border/countdown color */
  border: "#e63946",

  /** Text stroke color */
  textStroke: "#f6f6f6",

  /** Mid gray for UI elements */
  midGray: "#808080",
} as const;

/**
 * CSS variable names for accessing theme colors.
 * Use these when you need to reference colors in inline styles
 * that should respect dark mode and runtime theming.
 */
export const cssVars = {
  primary: "var(--color-logo-primary)",
  uiBackground: "var(--color-background-ui)",
  stroke: "var(--color-logo-stroke)",
  textStroke: "var(--color-text-stroke)",
  text: "var(--color-text)",
  background: "var(--color-background)",
  midGray: "var(--color-mid-gray)",
  bar: "var(--color-bar)",
  border: "var(--color-border)",
} as const;

/**
 * Complete theme export for convenient access.
 */
export const theme = {
  colors,
  cssVars,
} as const;

export default theme;
