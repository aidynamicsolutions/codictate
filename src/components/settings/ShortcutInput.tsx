import React from "react";
import { CodictateShortcut } from "./GlobalShortcutInput";

interface ShortcutInputProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  shortcutId: string;
  disabled?: boolean;
}

/**
 * Shortcut input component that uses the Tauri global shortcut implementation.
 */
export const ShortcutInput: React.FC<ShortcutInputProps> = (props) => {
  return <CodictateShortcut {...props} />;
};
